use crate::emulation::SyscallMock;
use crate::R;
use libc::{c_ulonglong, user_regs_struct};
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::sys::wait::{wait, waitpid, WaitStatus};
use nix::unistd::Pid;
use nix::unistd::{execv, fork, getpid, ForkResult};
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::{read_to_string, write};
use std::os::unix::ffi::OsStrExt;
use std::panic;
use std::path::Path;
use tempdir::TempDir;

#[derive(PartialEq, Debug, Clone, Eq, Hash)]
pub enum Syscall {
    Execve,
    Unknown(c_ulonglong),
}

impl From<user_regs_struct> for Syscall {
    fn from(registers: user_regs_struct) -> Self {
        if registers.orig_rax == libc::SYS_execve as c_ulonglong {
            Syscall::Execve
        } else {
            Syscall::Unknown(registers.orig_rax)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyscallStop {
    Enter,
    Exit,
}

pub struct Tracer {
    tracee_pid: Pid,
    syscall_mock: SyscallMock,
    entered_syscalls: HashMap<Pid, Syscall>,
}

impl Tracer {
    fn new(tracee_pid: Pid, syscall_mock: SyscallMock) -> Self {
        Tracer {
            tracee_pid,
            syscall_mock,
            entered_syscalls: HashMap::new(),
        }
    }

    pub fn run_against_mock<F>(executable: &Path, mk_syscall_mock: F) -> R<SyscallMock>
    where
        F: FnOnce(Pid) -> SyscallMock,
    {
        fork_with_child_errors(
            || {
                ptrace::traceme()?;
                signal::kill(getpid(), Some(Signal::SIGSTOP))?;
                let path = CString::new(executable.as_os_str().as_bytes())?;
                execv(&path, &[path.clone()])?;
                Ok(())
            },
            |tracee_pid: Pid| -> R<SyscallMock> {
                waitpid(tracee_pid, None)?;
                ptrace::setoptions(
                    tracee_pid,
                    Options::PTRACE_O_TRACESYSGOOD | Options::PTRACE_O_TRACEFORK,
                )?;
                ptrace::syscall(tracee_pid)?;

                let mut syscall_mock = Tracer::new(tracee_pid, mk_syscall_mock(tracee_pid));
                syscall_mock.trace()?;
                Ok(syscall_mock.syscall_mock)
            },
        )
    }

    fn trace(&mut self) -> R<()> {
        loop {
            let status = wait()?;
            self.handle_wait_status(status)?;
            match status {
                WaitStatus::Exited(pid, ..) => {
                    if self.tracee_pid == pid {
                        break;
                    }
                }
                _ => {
                    ptrace::syscall(status.pid().unwrap())?;
                }
            }
        }
        Ok(())
    }

    fn handle_wait_status(&mut self, status: WaitStatus) -> R<()> {
        if let WaitStatus::PtraceSyscall(pid, ..) = status {
            let registers = ptrace::getregs(pid)?;
            let syscall = Syscall::from(registers);
            let syscall_stop = self.update_syscall_state(pid, &syscall)?;
            self.syscall_mock
                .handle_syscall(pid, syscall_stop, syscall, registers)?;
        }
        Ok(())
    }

    fn update_syscall_state(&mut self, pid: Pid, syscall: &Syscall) -> R<SyscallStop> {
        Ok(match self.entered_syscalls.get(&pid) {
            None => {
                self.entered_syscalls.insert(pid, syscall.clone());
                SyscallStop::Enter
            }
            Some(old) => {
                if old != syscall {
                    Err("update_syscall_state: exiting with the wrong syscall")?
                } else {
                    self.entered_syscalls.remove(&pid);
                    SyscallStop::Exit
                }
            }
        })
    }
}

#[cfg(test)]
mod test_tracer {
    use super::*;

    mod update_syscall_state {
        use super::*;
        use std::collections::vec_deque::VecDeque;

        fn tracer() -> Tracer {
            let pid = Pid::from_raw(1);
            Tracer::new(pid, SyscallMock::new(pid, VecDeque::new()))
        }

        #[test]
        fn returns_entry_for_new_syscalls() -> R<()> {
            let mut tracer = tracer();
            assert_eq!(
                tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(23))?,
                SyscallStop::Enter
            );
            Ok(())
        }

        #[test]
        fn tracks_entry_and_exit_for_multiple_syscalls() -> R<()> {
            let mut tracer = tracer();
            tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(23))?;
            assert_eq!(
                tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(23))?,
                SyscallStop::Exit
            );
            assert_eq!(
                tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(23))?,
                SyscallStop::Enter
            );
            assert_eq!(
                tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(23))?,
                SyscallStop::Exit
            );
            Ok(())
        }

        mod when_a_different_process_is_inside_a_system_call {
            use super::*;

            #[test]
            fn tracks_entry_and_exit_for_multiple_syscalls() -> R<()> {
                let mut tracer = tracer();
                tracer.update_syscall_state(Pid::from_raw(42), &Syscall::Unknown(23))?;
                assert_eq!(
                    tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(23))?,
                    SyscallStop::Enter
                );
                assert_eq!(
                    tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(23))?,
                    SyscallStop::Exit
                );
                assert_eq!(
                    tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(23))?,
                    SyscallStop::Enter
                );
                assert_eq!(
                    tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(23))?,
                    SyscallStop::Exit
                );
                Ok(())
            }
        }

        #[test]
        fn complains_when_exiting_with_a_different_syscall() -> R<()> {
            let mut tracer = tracer();
            tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(1))?;
            assert_eq!(
                format!(
                    "{}",
                    tracer
                        .update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(2))
                        .unwrap_err()
                ),
                "update_syscall_state: exiting with the wrong syscall"
            );
            Ok(())
        }
    }
}

pub fn fork_with_child_errors<A>(
    child_action: impl FnOnce() -> R<()> + panic::UnwindSafe,
    parent_action: impl FnOnce(Pid) -> R<A>,
) -> R<A> {
    let tempdir = TempDir::new("check-protocols")?;
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
