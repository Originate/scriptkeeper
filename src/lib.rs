#![cfg_attr(feature = "ci", deny(warnings))]

use libc::{c_ulonglong, c_void};
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::sys::wait::{wait, waitpid, WaitStatus};
use nix::unistd::Pid;
use nix::unistd::{execv, fork, getpid, ForkResult};
use std::ffi::CString;
use std::fs::{read_to_string, write};
use std::os::unix::ffi::OsStrExt;
use std::panic;
use std::path::{Path, PathBuf};
use tempdir::TempDir;

pub type R<A> = Result<A, Box<std::error::Error>>;

fn ptrace_peekdata(pid: Pid, address: c_ulonglong) -> R<[u8; 8]> {
    unsafe {
        let word = ptrace::read(pid, address as *mut c_void)?;
        let ptr: &[u8; 8] = &*(&word as *const i64 as *const [u8; 8]);
        Ok(*ptr)
    }
}

fn ptrace_peekdata_iter(pid: Pid, address: c_ulonglong) -> impl Iterator<Item = R<[u8; 8]>> {
    struct Iter {
        pid: Pid,
        address: c_ulonglong,
    };

    impl Iterator for Iter {
        type Item = R<[u8; 8]>;

        fn next(&mut self) -> Option<Self::Item> {
            let result = ptrace_peekdata(self.pid, self.address);
            self.address += 8;
            Some(result)
        }
    }

    Iter { pid, address }
}

fn data_to_string(data: impl Iterator<Item = R<[u8; 8]>>) -> R<String> {
    let mut result = vec![];
    'outer: for word in data {
        for char in word?.iter() {
            if *char == 0 {
                break 'outer;
            }
            result.push(*char);
        }
    }
    Ok(String::from_utf8(result)?)
}

#[cfg(test)]
mod test_data_to_string {
    use super::*;

    #[test]
    fn reads_null_terminated_strings_from_one_word() {
        let data = vec![[102, 111, 111, 0, 0, 0, 0, 0]].into_iter().map(Ok);
        assert_eq!(data_to_string(data).unwrap(), "foo");
    }

    #[test]
    fn works_for_multiple_words() {
        let data = vec![
            [97, 98, 99, 100, 101, 102, 103, 104],
            [105, 0, 0, 0, 0, 0, 0, 0],
        ]
        .into_iter()
        .map(Ok);
        assert_eq!(data_to_string(data).unwrap(), "abcdefghi");
    }

    #[test]
    fn works_when_null_is_on_the_edge() {
        let data = vec![
            [97, 98, 99, 100, 101, 102, 103, 104],
            [0, 0, 0, 0, 0, 0, 0, 0],
        ]
        .into_iter()
        .map(Ok);
        assert_eq!(data_to_string(data).unwrap(), "abcdefgh");
    }
}

pub fn fork_with_child_errors<A>(
    child_action: impl FnOnce() -> R<()> + panic::UnwindSafe,
    parent_action: impl Fn(Pid) -> R<A>,
) -> R<A> {
    let tempdir = TempDir::new("tracing-poc")?;
    let error_file_path = tempdir.path().join("error");
    write(&error_file_path, "")?;
    match fork()? {
        ForkResult::Child => {
            Box::leak(Box::new(tempdir));
            let result: R<()> = (|| -> R<()> {
                match panic::catch_unwind(child_action) {
                    Ok(r) => r,
                    Err(err) => match err.downcast_ref::<&str>() {
                        None => Err("child panicked with an unsupported type")?,
                        Some(str) => Err(*str)?,
                    },
                }?;
                Err("child_action: please either exec or fail".to_string())?;
                Ok(())
            })();
            match result {
                Ok(()) => {}
                Err(error) => write(&error_file_path, format!("{}", error))?,
            };
            std::process::exit(0);
        }
        ForkResult::Parent { child } => {
            let result = parent_action(child);
            match read_to_string(&error_file_path)?.as_str() {
                "" => result,
                error => Err(error)?,
            }
        }
    }
}

#[cfg(test)]
mod test_fork_with_child_errors {
    use super::*;

    #[test]
    fn runs_the_child_action() -> R<()> {
        let tempdir = TempDir::new("test")?;
        let temp_file_path = tempdir.path().join("foo");
        fork_with_child_errors(
            || {
                write(&temp_file_path, "bar")?;
                execv(&CString::new("/bin/true")?, &vec![])?;
                Ok(())
            },
            |child: Pid| {
                loop {
                    match waitpid(child, None)? {
                        WaitStatus::Exited(..) => break,
                        _ => {}
                    }
                }
                Ok(())
            },
        )?;
        assert_eq!(read_to_string(&temp_file_path)?, "bar");
        Ok(())
    }

    #[test]
    fn raises_child_errors_in_the_parent() {
        let result: R<()> = fork_with_child_errors(
            || {
                Err("test error")?;
                Ok(())
            },
            |child: Pid| {
                loop {
                    match waitpid(child, None)? {
                        WaitStatus::Exited(..) => break,
                        _ => {}
                    }
                }
                Ok(())
            },
        );
        assert_eq!(format!("{}", result.unwrap_err()), "test error");
    }

    #[test]
    fn raises_child_panics_in_the_parent() {
        let result: R<()> = fork_with_child_errors(
            || {
                panic!("test panic");
            },
            |child: Pid| {
                loop {
                    match waitpid(child, None)? {
                        WaitStatus::Exited(..) => break,
                        _ => {}
                    }
                }
                Ok(())
            },
        );
        assert_eq!(format!("{}", result.unwrap_err()), "test panic");
    }

    #[test]
    fn raises_parent_errors_in_the_parent() {
        let result: R<()> = fork_with_child_errors(
            || {
                execv(&CString::new("/bin/true")?, &vec![])?;
                Ok(())
            },
            |child: Pid| {
                loop {
                    match waitpid(child, None)? {
                        WaitStatus::Exited(..) => break,
                        _ => {}
                    }
                }
                Err("test error")?;
                Ok(())
            },
        );
        assert_eq!(format!("{}", result.unwrap_err()), "test error");
    }

    #[test]
    fn raises_an_error_when_the_child_does_not_exec() {
        let result: R<()> = fork_with_child_errors(
            || Ok(()),
            |child: Pid| {
                loop {
                    match waitpid(child, None)? {
                        WaitStatus::Exited(..) => break,
                        _ => {}
                    }
                }
                Ok(())
            },
        );
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "child_action: please either exec or fail"
        );
    }

    #[test]
    fn child_errors_take_precedence_over_parent_errors() {
        let result: R<()> = fork_with_child_errors(
            || {
                Err("test child error")?;
                Ok(())
            },
            |child: Pid| {
                loop {
                    match waitpid(child, None)? {
                        WaitStatus::Exited(..) => break,
                        _ => {}
                    }
                }
                Err("test parent error")?;
                Ok(())
            },
        );
        assert_eq!(format!("{}", result.unwrap_err()), "test child error");
    }
}

pub fn first_execve_path(executable: &Path) -> R<PathBuf> {
    fork_with_child_errors(
        || {
            ptrace::traceme()?;
            signal::kill(getpid(), Some(Signal::SIGSTOP))?;
            let path = CString::new(executable.as_os_str().as_bytes())?;
            execv(&path, &[path.clone()])?;
            Ok(())
        },
        |child: Pid| -> R<PathBuf> {
            let mut result = None;
            waitpid(child, None)?;
            ptrace::setoptions(
                child,
                Options::PTRACE_O_TRACESYSGOOD | Options::PTRACE_O_TRACEFORK,
            )?;
            ptrace::syscall(child)?;

            loop {
                let status = wait()?;
                if let WaitStatus::PtraceSyscall(pid, ..) = status {
                    let registers = ptrace::getregs(pid)?;
                    if registers.orig_rax == libc::SYS_execve as c_ulonglong
                        && registers.rdi > 0
                        && result.is_none()
                        && child != pid
                    {
                        let path = data_to_string(ptrace_peekdata_iter(pid, registers.rdi))?;
                        result = Some(PathBuf::from(path));
                    }
                }
                match status {
                    WaitStatus::Exited(pid, ..) => {
                        if child == pid {
                            break;
                        }
                    }
                    _ => {
                        ptrace::syscall(status.pid().unwrap())?;
                    }
                }
            }
            Ok(result.ok_or("execve didn't happen")?)
        },
    )
}

#[cfg(test)]
mod test_first_execve_path {
    use super::*;
    use std::fs::copy;
    use std::process::Command;

    struct TempFile {
        tempdir: TempDir,
    }

    impl TempFile {
        fn new() -> R<TempFile> {
            let tempdir = TempDir::new("test")?;
            Ok(TempFile { tempdir })
        }

        fn path(&self) -> PathBuf {
            self.tempdir.path().join("file")
        }
    }

    fn run(command: &str, args: Vec<&str>) -> R<()> {
        let status = Command::new(command).args(args).status()?;
        if status.success() {
            Ok(())
        } else {
            Err("command failed")?
        }
    }

    fn write_temp_script(script: &str) -> R<TempFile> {
        let tempfile = TempFile::new()?;
        write(&tempfile.path(), script.trim_start())?;
        run("chmod", vec!["+x", tempfile.path().to_str().unwrap()])?;
        Ok(tempfile)
    }

    #[test]
    fn returns_the_path_of_the_first_executable_spawned_by_the_script() -> R<()> {
        let script = write_temp_script(
            r##"
                #!/usr/bin/env bash

                cd /bin
                ./true
            "##,
        )?;
        assert_eq!(first_execve_path(&script.path())?, PathBuf::from("./true"));
        Ok(())
    }

    #[test]
    fn works_for_longer_file_names() -> R<()> {
        let long_command = TempFile::new()?;
        copy("/bin/true", long_command.path())?;
        let script = write_temp_script(&format!(
            r##"
                #!/usr/bin/env bash

                {}
            "##,
            long_command.path().to_str().unwrap()
        ))?;
        assert_eq!(first_execve_path(&script.path())?, long_command.path());
        Ok(())
    }

    #[test]
    fn complains_when_the_file_does_not_exist() {
        assert_eq!(
            format!(
                "{}",
                first_execve_path(Path::new("./does_not_exist")).unwrap_err()
            ),
            "ENOENT: No such file or directory"
        );
    }
}
