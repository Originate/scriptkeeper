pub mod checker_result;
pub mod executable_mock;

use crate::context::Context;
use crate::test_spec;
use crate::test_spec::{
    executable_path::{is_unmocked_command, should_assume_in_path},
    Test,
};
use crate::tracer::stdio_redirecting::Redirector;
use crate::tracer::{tracee_memory, SyscallMock};
use crate::utils::short_temp_files::ShortTempFile;
use crate::R;
use checker_result::CheckerResult;
use libc::{c_ulonglong, mode_t, user_regs_struct};
use nix::sys::ptrace;
use nix::unistd::Pid;
use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

#[derive(Debug)]
pub struct TestChecker {
    context: Context,
    pub test: Test,
    pub unmocked_commands: Vec<PathBuf>,
    pub result: CheckerResult,
    temporary_executables: Vec<ShortTempFile>,
}

impl TestChecker {
    pub fn new(context: &Context, test: Test, unmocked_commands: &[PathBuf]) -> TestChecker {
        TestChecker {
            context: context.clone(),
            test,
            unmocked_commands: unmocked_commands.to_vec(),
            result: CheckerResult::Pass,
            temporary_executables: vec![],
        }
    }

    fn allow_failing_scripts_to_continue() -> executable_mock::Config {
        executable_mock::Config {
            stdout: vec![],
            exitcode: 0,
        }
    }

    fn handle_step(&mut self, received: test_spec::Command) -> R<PathBuf> {
        let mock_config = match self.test.steps.pop_front() {
            Some(next_test_step) => {
                if !next_test_step.command_matcher.matches(&received) {
                    self.register_step_error(
                        &next_test_step.command_matcher.format(),
                        &received.format(),
                    );
                }
                executable_mock::Config {
                    stdout: next_test_step.stdout,
                    exitcode: next_test_step.exitcode,
                }
            }
            None => {
                self.register_step_error("<script termination>", &received.format());
                TestChecker::allow_failing_scripts_to_continue()
            }
        };
        let mock_executable_contents =
            executable_mock::create_mock_executable(&self.context, mock_config)?;
        let temp_executable = ShortTempFile::new(&mock_executable_contents)?;
        let path = temp_executable.path();
        self.temporary_executables.push(temp_executable);
        Ok(path)
    }

    fn mock_file(pid: Pid, registers: &user_regs_struct, mock_mode: mode_t) -> R<()> {
        let statbuf_ptr = registers.rsi;
        #[allow(clippy::forget_copy)]
        tracee_memory::poke_four_bytes(
            pid,
            statbuf_ptr + (offset_of!(libc::stat, st_mode) as u64),
            mock_mode as u32,
        )?;
        let mut registers = *registers;
        registers.rax = 0;
        ptrace::setregs(pid, registers)?;
        Ok(())
    }

    fn register_step_error(&mut self, expected: &str, received: &str) {
        self.register_error(format!(
            "  expected: {}\n  received: {}\n",
            expected, received
        ));
    }

    fn register_error(&mut self, message: String) {
        match self.result {
            CheckerResult::Pass => {
                self.result = CheckerResult::Failure(message);
            }
            CheckerResult::Failure(_) => {}
        }
    }
}

impl SyscallMock for TestChecker {
    type Result = CheckerResult;

    fn handle_execve_enter(
        &mut self,
        pid: Pid,
        registers: &user_regs_struct,
        executable: PathBuf,
        arguments: Vec<OsString>,
    ) -> R<()> {
        if !is_unmocked_command(&self.unmocked_commands, &executable) {
            let mock_executable_path = self.handle_step(test_spec::Command {
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
        if let Some(mock_cwd) = &self.test.cwd {
            let mock_cwd = mock_cwd.as_os_str().as_bytes();
            let buffer_ptr = registers.rdi;
            let max_size = registers.rsi;
            tracee_memory::poke_string(pid, buffer_ptr, mock_cwd, max_size)?;
            let mut registers = *registers;
            registers.rax = mock_cwd.len() as c_ulonglong + 1;
            ptrace::setregs(pid, registers)?;
        }
        Ok(())
    }

    fn handle_stat_exit(&self, pid: Pid, registers: &user_regs_struct, filename: PathBuf) -> R<()> {
        if self.test.mocked_files.contains(&filename) {
            let mock_mode = if filename.as_os_str().as_bytes().ends_with(b"/") {
                libc::S_IFDIR
            } else {
                libc::S_IFREG
            };
            TestChecker::mock_file(pid, registers, mock_mode)?;
        } else if should_assume_in_path(&filename) {
            TestChecker::mock_file(pid, registers, libc::S_IFREG)?;
        }
        Ok(())
    }

    fn handle_end(mut self, exitcode: i32, redirector: &Redirector) -> R<CheckerResult> {
        if let Some(expected_step) = self.test.steps.pop_front() {
            self.register_step_error(
                &expected_step.command_matcher.format(),
                "<script terminated>",
            );
        }
        let expected_exitcode = self.test.exitcode.unwrap_or(0);
        if exitcode != expected_exitcode {
            self.register_step_error(
                &format!("<exitcode {}>", expected_exitcode),
                &format!("<exitcode {}>", exitcode),
            );
        }
        if let Some(expected_stderr) = &self.test.stderr {
            match redirector.stderr.captured()? {
                None => panic!("scriptkeeper bug: stderr expected, but not captured"),
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
