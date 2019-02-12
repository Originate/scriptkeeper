#![cfg_attr(feature = "ci", deny(warnings))]

mod tracee_memory;

use crate::tracee_memory::{data_to_string, ptrace_peekdata_iter, ptrace_pokedata, string_to_data};
use libc::c_ulonglong;
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::sys::wait::{wait, waitpid, WaitStatus};
use nix::unistd::Pid;
use nix::unistd::{execv, fork, getpid, ForkResult};
use std::ffi::CString;
use std::fs::{copy, read_to_string, write};
use std::os::unix::ffi::OsStrExt;
use std::panic;
use std::path::{Path, PathBuf};
use tempdir::TempDir;

pub type R<A> = Result<A, Box<std::error::Error>>;

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

fn get_execve_path(status: &WaitStatus, script_pid: Pid) -> R<Option<PathBuf>> {
    if let WaitStatus::PtraceSyscall(pid, ..) = status {
        let registers = ptrace::getregs(*pid)?;
        if registers.orig_rax == libc::SYS_execve as c_ulonglong
            && registers.rdi > 0
            && script_pid != *pid
        {
            let path = data_to_string(ptrace_peekdata_iter(*pid, registers.rdi))?;
            copy("/bin/true", "/tmp/a")?;
            ptrace_pokedata(*pid, registers.rdi, string_to_data("/tmp/a")?)?;
            Ok(Some(PathBuf::from(path)))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

pub fn execve_paths(executable: &Path) -> R<Vec<PathBuf>> {
    fork_with_child_errors(
        || {
            ptrace::traceme()?;
            signal::kill(getpid(), Some(Signal::SIGSTOP))?;
            let path = CString::new(executable.as_os_str().as_bytes())?;
            execv(&path, &[path.clone()])?;
            Ok(())
        },
        |script_pid: Pid| -> R<Vec<PathBuf>> {
            let mut result = vec![];
            waitpid(script_pid, None)?;
            ptrace::setoptions(
                script_pid,
                Options::PTRACE_O_TRACESYSGOOD | Options::PTRACE_O_TRACEFORK,
            )?;
            ptrace::syscall(script_pid)?;

            loop {
                let status = wait()?;
                if let Some(path) = get_execve_path(&status, script_pid)? {
                    result.push(path)
                };
                match status {
                    WaitStatus::Exited(pid, ..) => {
                        if script_pid == pid {
                            break;
                        }
                    }
                    _ => {
                        ptrace::syscall(status.pid().unwrap())?;
                    }
                }
            }
            Ok(result)
        },
    )
}

#[cfg(test)]
mod test_execve_paths {
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
        assert_eq!(
            execve_paths(&script.path())?.first().unwrap(),
            &PathBuf::from("./true")
        );
        Ok(())
    }

    #[test]
    fn returns_multiple_executables_spawned_by_the_script() -> R<()> {
        let script = write_temp_script(
            r##"
                #!/usr/bin/env bash

                cd /bin
                ./true
                ./false
            "##,
        )?;
        assert_eq!(
            execve_paths(&script.path())?,
            vec![PathBuf::from("./true"), PathBuf::from("./false")]
        );
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
        assert_eq!(execve_paths(&script.path())?, vec![long_command.path()]);
        Ok(())
    }

    #[test]
    fn complains_when_the_file_does_not_exist() {
        assert_eq!(
            format!(
                "{}",
                execve_paths(Path::new("./does_not_exist")).unwrap_err()
            ),
            "ENOENT: No such file or directory"
        );
    }

    #[test]
    fn does_not_execute_the_commands() -> R<()> {
        let testfile = TempFile::new()?;
        let script = write_temp_script(&format!(
            r##"
                #!/usr/bin/env bash

                touch {}
            "##,
            testfile.path().to_str().ok_or("utf8 error")?
        ))?;
        execve_paths(&script.path())?;
        assert!(!testfile.path().exists(), "touch was executed");
        Ok(())
    }
}

pub fn run(script: &Path) -> R<String> {
    Ok(format!("executables: {:?}\n", execve_paths(script)?))
}
