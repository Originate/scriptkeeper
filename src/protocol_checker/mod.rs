pub mod executable_mock;
pub mod test_result;

use crate::context::Context;
use crate::protocol;
use crate::protocol::Protocol;
use crate::tracer::stdio_redirecting::Redirector;
use crate::tracer::{tracee_memory, SyscallMock};
use crate::utils::short_temp_files::ShortTempFile;
use crate::R;
use libc::{c_ulonglong, user_regs_struct};
use nix::sys::ptrace;
use nix::unistd::Pid;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use test_result::TestResult;

#[derive(Debug)]
pub struct ProtocolChecker {
    context: Context,
    protocol: Protocol,
    unmocked_commands: Vec<Vec<u8>>,
    pub result: TestResult,
    temporary_executables: Vec<ShortTempFile>,
}

impl ProtocolChecker {
    pub fn new(
        context: &Context,
        protocol: Protocol,
        unmocked_commands: &[Vec<u8>],
    ) -> ProtocolChecker {
        ProtocolChecker {
            context: context.clone(),
            protocol,
            unmocked_commands: unmocked_commands.to_vec(),
            result: TestResult::Pass,
            temporary_executables: vec![],
        }
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
                    self.register_step_error(
                        &next_protocol_step.command.format(),
                        &received.format(),
                    );
                }
                executable_mock::Config {
                    stdout: next_protocol_step.stdout,
                    exitcode: next_protocol_step.exitcode,
                }
            }
            None => {
                self.register_step_error("<protocol end>", &received.format());
                ProtocolChecker::allow_failing_scripts_to_continue()
            }
        };
        let mock_executable_contents =
            executable_mock::create_mock_executable(&self.context, mock_config)?;
        let temp_executable = ShortTempFile::new(&mock_executable_contents)?;
        let path = temp_executable.path();
        self.temporary_executables.push(temp_executable);
        Ok(path)
    }

    fn register_step_error(&mut self, expected: &str, received: &str) {
        self.register_error(format!(
            "  expected: {}\n  received: {}\n",
            expected, received
        ));
    }

    fn register_error(&mut self, message: String) {
        match self.result {
            TestResult::Pass => {
                self.result = TestResult::Failure(message);
            }
            TestResult::Failure(_) => {}
        }
    }
}

impl SyscallMock for ProtocolChecker {
    type Result = TestResult;

    fn handle_execve_enter(
        &mut self,
        pid: Pid,
        registers: &user_regs_struct,
        executable: Vec<u8>,
    ) -> R<()> {
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
        Ok(())
    }

    fn handle_getcwd_exit(&self, pid: Pid, registers: &user_regs_struct) -> R<()> {
        if let Some(mock_cwd) = &self.protocol.cwd {
            let buffer_ptr = registers.rdi;
            let max_size = registers.rsi;
            tracee_memory::poke_string(pid, buffer_ptr, mock_cwd, max_size)?;
            let mut registers = *registers;
            registers.rax = mock_cwd.len() as c_ulonglong + 1;
            ptrace::setregs(pid, registers)?;
        }
        Ok(())
    }

    fn handle_stat_exit(&self, pid: Pid, registers: &user_regs_struct, filename: Vec<u8>) -> R<()> {
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
        Ok(())
    }

    fn handle_end(mut self, exitcode: i32, redirector: &Redirector) -> R<TestResult> {
        if let Some(expected_step) = self.protocol.steps.pop_front() {
            self.register_step_error(&expected_step.command.format(), "<script terminated>");
        }
        if exitcode != self.protocol.exitcode {
            self.register_step_error(
                &format!("<exitcode {}>", self.protocol.exitcode),
                &format!("<exitcode {}>", exitcode),
            );
        }
        if let Some(expected_stderr) = &self.protocol.stderr {
            match redirector.stderr.captured()? {
                None => panic!("check-protocols bug: stderr expected, but not captured"),
                Some(captured_stderr) => {
                    if &captured_stderr != expected_stderr {
                        self.register_error(format!(
                            "  expected output to stderr: {:?}\
                             \n  received output to stderr: {:?}\n",
                            String::from_utf8_lossy(&expected_stderr).as_ref(),
                            String::from_utf8_lossy(&captured_stderr).as_ref(),
                        ));
                    }
                }
            }
        }
        Ok(self.result)
    }
}