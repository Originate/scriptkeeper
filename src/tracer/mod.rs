mod debugging;
pub mod stdio_redirecting;
pub mod syscall;
pub mod tracee_memory;

use crate::context::Context;
use crate::utils::parse_hashbang;
use crate::R;
use debugging::Debugger;
use libc::user_regs_struct;
use nix;
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::sys::wait::{wait, waitpid, WaitStatus};
use nix::unistd::{execve, fork, getpid, ForkResult, Pid};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::panic;
use std::path::{Path, PathBuf};
use std::str;
use stdio_redirecting::{CaptureStderr, Redirector};
use syscall::Syscall;
use tempdir::TempDir;

pub trait SyscallMock {
    type Result;

    fn handle_execve_enter(
        &mut self,
        _pid: Pid,
        _registers: &user_regs_struct,
        _executable: Vec<u8>,
        _arguments: Vec<Vec<u8>>,
    ) -> R<()> {
        Ok(())
    }

    fn handle_exited(&mut self, _pid: Pid, _exitcode: i32) -> R<()> {
        Ok(())
    }

    fn handle_getcwd_exit(&self, _pid: Pid, _registers: &user_regs_struct) -> R<()> {
        Ok(())
    }

    fn handle_stat_exit(
        &self,
        _pid: Pid,
        _registers: &user_regs_struct,
        _filename: Vec<u8>,
    ) -> R<()> {
        Ok(())
    }

    fn handle_end(self, exitcode: i32, redirector: &Redirector) -> R<Self::Result>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyscallStop {
    Enter,
    Exit,
}

pub struct Tracer {
    tracee_pid: Pid,
    entered_syscalls: HashMap<Pid, Syscall>,
}

impl Tracer {
    fn new(tracee_pid: Pid) -> Self {
        Tracer {
            tracee_pid,
            entered_syscalls: HashMap::new(),
        }
    }

    fn execve_params(
        interpreter: &Option<PathBuf>,
        program: &Path,
        args: Vec<String>,
        env: HashMap<String, String>,
    ) -> R<(CString, Vec<CString>, Vec<CString>)> {
        let c_executable = CString::new(program.as_os_str().as_bytes())?;
        let mut c_args = VecDeque::new();
        c_args.push_back(c_executable.clone());
        for arg in &args {
            c_args.push_back(CString::new(arg.clone())?);
        }
        let mut c_env = vec![];
        for (key, value) in env {
            c_env.push(CString::new(format!("{}={}", key, value))?);
        }
        Ok(match interpreter {
            Some(interpreter) => {
                let c_interpreter = CString::new(interpreter.as_os_str().as_bytes())?;
                c_args.push_front(c_interpreter.clone());
                (c_interpreter, c_args.into_iter().collect(), c_env)
            }
            None => (c_executable.clone(), c_args.into_iter().collect(), c_env),
        })
    }

    fn format_execve_error(
        error: nix::Error,
        interpreter: &Option<PathBuf>,
        program: &Path,
    ) -> String {
        let (program, interpreter) = if let Some(interpreter) = interpreter {
            (program, interpreter.to_string_lossy().into_owned())
        } else {
            (
                program,
                parse_hashbang(program).unwrap_or_else(|| "your interpreter".to_string()),
            )
        };
        let hint = format!("Does {} exist?", interpreter);
        format!(
            "execve'ing {} failed with error: {}\n{}",
            program.to_string_lossy(),
            error,
            hint,
        )
    }

    fn execve(
        interpreter: &Option<PathBuf>,
        program: &Path,
        args: Vec<String>,
        env: HashMap<String, String>,
    ) -> R<()> {
        let (c_executable, c_args, c_env) = Tracer::execve_params(interpreter, program, args, env)?;
        execve(&c_executable, &c_args, &c_env)
            .map_err(|error| Self::format_execve_error(error, interpreter, program))?;
        Ok(())
    }

    pub fn run_against_mock<MockResult>(
        context: &Context,
        interpreter: &Option<PathBuf>,
        program: &Path,
        args: Vec<String>,
        env: HashMap<String, String>,
        capture_stderr: CaptureStderr,
        mut syscall_mock: impl SyscallMock<Result = MockResult>,
    ) -> R<MockResult> {
        let redirector = Redirector::new(context, capture_stderr)?;
        fork_with_child_errors(
            || {
                redirector.child_redirect_streams()?;
                ptrace::traceme().map_err(|error| format!("PTRACE_TRACEME failed: {}", error))?;
                signal::kill(getpid(), Some(Signal::SIGSTOP))?;
                Tracer::execve(interpreter, program, args, env)?;
                Ok(())
            },
            |tracee_pid: Pid| -> R<MockResult> {
                let join = redirector.parent_relay_streams()?;
                waitpid(tracee_pid, None)?;
                ptrace::setoptions(
                    tracee_pid,
                    Options::PTRACE_O_TRACESYSGOOD
                        | Options::PTRACE_O_TRACEFORK
                        | Options::PTRACE_O_TRACEVFORK,
                )?;
                ptrace::syscall(tracee_pid)?;
                let mut tracer = Tracer::new(tracee_pid);
                let exitcode = tracer.trace(&mut syscall_mock)?;
                join()?;
                syscall_mock.handle_end(exitcode, &redirector)
            },
        )
    }

    fn trace<MockResult>(&mut self, syscall_mock: &mut SyscallMock<Result = MockResult>) -> R<i32> {
        let mut debugger = Debugger::new();
        Ok(loop {
            let status = wait()?;
            self.handle_wait_status(&mut debugger, syscall_mock, status)?;
            match status {
                WaitStatus::Exited(pid, exitcode) => {
                    if self.tracee_pid == pid {
                        break exitcode;
                    } else {
                        syscall_mock.handle_exited(pid, exitcode)?;
                    }
                }
                _ => {
                    ptrace::syscall(status.pid().unwrap())?;
                }
            }
        })
    }

    fn handle_wait_status<MockResult>(
        &mut self,
        debugger: &mut Debugger,
        syscall_mock: &mut SyscallMock<Result = MockResult>,
        status: WaitStatus,
    ) -> R<()> {
        if let WaitStatus::PtraceSyscall(pid, ..) = status {
            let registers = ptrace::getregs(pid)?;
            let syscall = Syscall::from(registers);
            let syscall_stop = self.update_syscall_state(pid, &syscall)?;
            debugger.log_syscall(pid, &syscall_stop, &syscall, || -> R<()> {
                self.handle_syscall(syscall_mock, pid, &syscall_stop, &syscall, &registers)
            })?;
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

    fn handle_syscall<MockResult>(
        &mut self,
        syscall_mock: &mut SyscallMock<Result = MockResult>,
        pid: Pid,
        syscall_stop: &SyscallStop,
        syscall: &Syscall,
        registers: &user_regs_struct,
    ) -> R<()> {
        match (&syscall, syscall_stop) {
            (Syscall::Execve, SyscallStop::Enter) => {
                if self.tracee_pid != pid {
                    let executable = tracee_memory::peek_string(pid, registers.rdi)?;
                    let arguments = tracee_memory::peek_string_array(pid, registers.rsi)?;
                    syscall_mock.handle_execve_enter(pid, registers, executable, arguments)?;
                }
            }
            (Syscall::Getcwd, SyscallStop::Exit) => {
                syscall_mock.handle_getcwd_exit(pid, registers)?
            }
            (Syscall::Stat, SyscallStop::Exit) => {
                let filename = tracee_memory::peek_string(pid, registers.rdi)?;
                syscall_mock.handle_stat_exit(pid, registers, filename)?
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod test_tracer {
    use super::*;

    mod update_syscall_state {
        use super::*;
        use test_utils::assert_error;

        fn tracer() -> Tracer {
            Tracer::new(Pid::from_raw(1))
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
            assert_error!(
                tracer.update_syscall_state(Pid::from_raw(2), &Syscall::Unknown(2)),
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
    fs::write(&error_file_path, "")?;
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
                Err(error) => fs::write(&error_file_path, format!("{}", error))?,
            };
            std::process::exit(0);
        }
        ForkResult::Parent { child } => {
            let result = parent_action(child);
            match fs::read_to_string(&error_file_path)?.as_str() {
                "" => result,
                error => Err(error)?,
            }
        }
    }
}

#[cfg(test)]
mod test_fork_with_child_errors {
    use super::*;
    use nix::unistd::execv;
    use test_utils::assert_error;

    #[test]
    fn runs_the_child_action() -> R<()> {
        let tempdir = TempDir::new("test")?;
        let temp_file_path = tempdir.path().join("foo");
        fork_with_child_errors(
            || {
                fs::write(&temp_file_path, "bar")?;
                execv(&CString::new("/bin/true")?, &[])?;
                Ok(())
            },
            |child: Pid| {
                loop {
                    if let WaitStatus::Exited(..) = waitpid(child, None)? {
                        break;
                    }
                }
                Ok(())
            },
        )?;
        assert_eq!(fs::read_to_string(&temp_file_path)?, "bar");
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
                    if let WaitStatus::Exited(..) = waitpid(child, None)? {
                        break;
                    }
                }
                Ok(())
            },
        );
        assert_error!(result, "test error");
    }

    #[test]
    fn raises_child_panics_in_the_parent() {
        let result: R<()> = fork_with_child_errors(
            || {
                panic!("test panic");
            },
            |child: Pid| {
                loop {
                    if let WaitStatus::Exited(..) = waitpid(child, None)? {
                        break;
                    }
                }
                Ok(())
            },
        );
        assert_error!(result, "test panic");
    }

    #[test]
    fn raises_parent_errors_in_the_parent() {
        let result: R<()> = fork_with_child_errors(
            || {
                execv(&CString::new("/bin/true")?, &[])?;
                Ok(())
            },
            |child: Pid| {
                loop {
                    if let WaitStatus::Exited(..) = waitpid(child, None)? {
                        break;
                    }
                }
                Err("test error")?;
                Ok(())
            },
        );
        assert_error!(result, "test error");
    }

    #[test]
    fn raises_an_error_when_the_child_does_not_exec() {
        let result: R<()> = fork_with_child_errors(
            || Ok(()),
            |child: Pid| {
                loop {
                    if let WaitStatus::Exited(..) = waitpid(child, None)? {
                        break;
                    }
                }
                Ok(())
            },
        );
        assert_error!(result, "child_action: please either exec or fail");
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
                    if let WaitStatus::Exited(..) = waitpid(child, None)? {
                        break;
                    }
                }
                Err("test parent error")?;
                Ok(())
            },
        );
        assert_error!(result, "test child error");
    }
}
