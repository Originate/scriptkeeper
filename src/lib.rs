#![cfg_attr(feature = "ci", deny(warnings))]

use libc::{c_long, c_ulonglong, pid_t};
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::Pid;
use nix::unistd::{execv, fork, getpid, ForkResult};
use std::ffi::CString;
use std::fs::{read_to_string, write};
use std::os::unix::ffi::OsStrExt;
use std::panic;
use std::path::Path;
use tempdir::TempDir;

pub type R<A> = Result<A, Box<std::error::Error>>;

extern "C" {
    fn c_ptrace_peekdata(pid: pid_t, address: c_long) -> c_long;
}

fn ptrace_peekdata(pid: Pid, address: c_ulonglong) -> [u8; 8] {
    unsafe {
        let word = c_ptrace_peekdata(pid.as_raw(), address as c_long);
        let ptr: &[u8; 8] = &*(&word as *const i64 as *const [u8; 8]);
        *ptr
    }
}

fn data_to_string(data: [u8; 8]) -> R<String> {
    let mut result = vec![];
    for char in data.iter() {
        if *char == 0 {
            break;
        }
        result.push(*char);
    }
    Ok(String::from_utf8(result)?)
}

#[cfg(test)]
mod test_data_to_string {
    use super::*;

    #[test]
    fn reads_null_terminated_strings() {
        let data = [102, 111, 111, 0, 0, 0, 0, 0];
        assert_eq!(data_to_string(data).unwrap(), "foo");
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
            let result = parent_action(child)?;
            match read_to_string(&error_file_path)?.as_str() {
                "" => Ok(result),
                error => Err(error)?,
            }
        }
    }
}

#[cfg(test)]
mod test_fork_with_child_errors {
    use super::*;
    use std::fs::{read_to_string, write};
    use tempdir::TempDir;

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
            |_child: Pid| {
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
}

pub fn first_execve_path(working_dir: Option<&Path>, executable: &Path) -> R<String> {
    fork_with_child_errors(
        || {
            match working_dir {
                None => {}
                Some(dir) => {
                    std::env::set_current_dir(dir)?;
                }
            }
            ptrace::traceme()?;
            signal::kill(getpid(), Some(Signal::SIGSTOP))?;
            let path = CString::new(executable.as_os_str().as_bytes())?;
            execv(&path, &[path.clone()])?;
            Ok(())
        },
        |child: Pid| -> R<String> {
            let mut result = None;
            waitpid(child, None)?;
            ptrace::setoptions(child, Options::PTRACE_O_TRACESYSGOOD)?;
            ptrace::syscall(child)?;

            loop {
                match waitpid(child, None)? {
                    WaitStatus::Exited(..) => break,
                    WaitStatus::PtraceSyscall(..) => {
                        if result.is_none() {
                            let registers = ptrace::getregs(child)?;
                            if registers.orig_rax == libc::SYS_execve as c_ulonglong
                                && registers.rdi > 0
                            {
                                let word = ptrace_peekdata(child, registers.rdi);
                                result = Some(data_to_string(word)?);
                            }
                        }
                    }
                    _ => {}
                }
                ptrace::syscall(child)?;
            }
            Ok(result.ok_or("execve didn't happen")?)
        },
    )
}

#[cfg(test)]
mod test_first_execve_path {
    use super::*;
    use std::process::Command;

    fn run(command: &str, args: Vec<&str>) -> R<()> {
        let status = Command::new(command).args(args).status()?;
        if status.success() {
            Ok(())
        } else {
            Err("command failed")?
        }
    }

    #[test]
    fn returns_the_path_of_the_spawned_executable() -> R<()> {
        let tempdir = TempDir::new("test")?;
        let script_path = tempdir.path().join("foo");
        write(
            &script_path,
            r##"
                #!/usr/bin/env bash

                echo executing foo script > /dev/null
            "##
            .trim_start(),
        )?;
        run("chmod", vec!["+x", script_path.to_str().unwrap()])?;
        assert_eq!(
            first_execve_path(Some(tempdir.path()), Path::new("./foo"))?,
            "./foo"
        );
        Ok(())
    }

    #[test]
    fn complains_when_the_file_does_not_exist() {
        assert_eq!(
            format!(
                "{}",
                first_execve_path(None, Path::new("./does_not_exist")).unwrap_err()
            ),
            "ENOENT: No such file or directory"
        );
    }
}
