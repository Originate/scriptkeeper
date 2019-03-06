pub mod executable_mock;
pub mod test_result;

use crate::context::Context;
use crate::protocol;
use crate::protocol::Protocol;
use crate::tracer::syscall::Syscall;
use crate::tracer::{tracee_memory, SyscallStop};
use crate::utils::short_temp_files::ShortTempFile;
use crate::R;
use libc::{c_ulonglong, user_regs_struct};
use nix::sys::ptrace;
use nix::unistd::Pid;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use test_result::TestResult;

#[derive(Debug)]
pub struct SyscallMock {
    context: Context,
    tracee_pid: Pid,
    protocol: Protocol,
    unmocked_commands: Vec<Vec<u8>>,
    pub result: TestResult,
    temporary_executables: Vec<ShortTempFile>,
}

impl SyscallMock {
    pub fn new(
        context: Context,
        tracee_pid: Pid,
        protocol: Protocol,
        unmocked_commands: &[Vec<u8>],
    ) -> SyscallMock {
        SyscallMock {
            context,
            tracee_pid,
            protocol,
            unmocked_commands: unmocked_commands.to_vec(),
            result: TestResult::Pass,
            temporary_executables: vec![],
        }
    }

    pub fn handle_syscall(
        &mut self,
        pid: Pid,
        syscall_stop: &SyscallStop,
        syscall: &Syscall,
        registers: &user_regs_struct,
    ) -> R<()> {
        match (&syscall, syscall_stop) {
            (Syscall::Execve, SyscallStop::Enter) => {
                if self.tracee_pid != pid {
                    let executable = tracee_memory::peek_string(pid, registers.rdi)?;
                    if !self.unmocked_commands.contains(&executable) {
                        let arguments = tracee_memory::peek_string_array(pid, registers.rsi)?;
                        let mock_executable_path = self.handle_step(protocol::Command {
                            executable,
                            arguments,
                        })?;
                        tracee_memory::poke_single_word_string(
                            pid,
                            registers.rdi,
                            &mock_executable_path.as_os_str().as_bytes(),
                        )?;
                    }
                }
            }
            (Syscall::Getcwd, SyscallStop::Exit) => {
                if let Some(mock_cwd) = &self.protocol.cwd {
                    let buffer_ptr = registers.rdi;
                    let max_size = registers.rsi;
                    tracee_memory::poke_string(pid, buffer_ptr, mock_cwd, max_size)?;
                    let mut registers = *registers;
                    registers.rax = mock_cwd.len() as c_ulonglong + 1;
                    ptrace::setregs(pid, registers)?;
                }
            }
            (Syscall::Stat, SyscallStop::Exit) => {
                let filename = tracee_memory::peek_string(pid, registers.rdi)?;
                if self.protocol.mocked_files.contains(&filename) {
                    let statbuf_ptr = registers.rsi;
                    let mock_mode = if filename.ends_with(b"/") {
                        libc::S_IFDIR
                    } else {
                        libc::S_IFREG
                    };
                    #[allow(clippy::forget_copy)]
                    tracee_memory::poke_four_bytes(
                        pid,
                        statbuf_ptr + (offset_of!(libc::stat, st_mode) as u64),
                        mock_mode as u32,
                    )?;
                    let mut registers = *registers;
                    registers.rax = 0;
                    ptrace::setregs(pid, registers)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn allow_failing_scripts_to_continue() -> executable_mock::Config {
        executable_mock::Config {
            stdout: vec![],
            exitcode: 0,
        }
    }

    fn handle_step(&mut self, received: protocol::Command) -> R<PathBuf> {
        let mock_config = match self.protocol.steps.pop_front() {
            Some(next_protocol_step) => {
                if next_protocol_step.command != received {
                    self.register_error(&next_protocol_step.command.format(), &received.format());
                }
                executable_mock::Config {
                    stdout: next_protocol_step.stdout,
                    exitcode: next_protocol_step.exitcode,
                }
            }
            None => {
                self.register_error("<protocol end>", &received.format());
                SyscallMock::allow_failing_scripts_to_continue()
            }
        };
        let mock_executable_contents =
            executable_mock::create_mock_executable(&self.context, mock_config)?;
        let temp_executable = ShortTempFile::new(&mock_executable_contents)?;
        let path = temp_executable.path();
        self.temporary_executables.push(temp_executable);
        Ok(path)
    }

    pub fn handle_end(mut self, exitcode: i32) -> TestResult {
        if let Some(expected_step) = self.protocol.steps.pop_front() {
            self.register_error(&expected_step.command.format(), "<script terminated>");
        }
        if exitcode != self.protocol.exitcode {
            self.register_error(
                &format!("<exitcode {}>", self.protocol.exitcode),
                &format!("<exitcode {}>", exitcode),
            );
        }
        self.result
    }

    fn register_error(&mut self, expected: &str, received: &str) {
        match self.result {
            TestResult::Pass => {
                self.result = TestResult::Failure(format!(
                    "  expected: {}\n  received: {}\n",
                    expected, received
                ));
            }
            TestResult::Failure(_) => {}
        }
    }
}
