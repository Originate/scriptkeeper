pub mod executable_mock;
pub mod test_result;

use crate::protocol;
use crate::protocol::Protocol;
use crate::syscall_mocking::syscall::Syscall;
use crate::syscall_mocking::{tracee_memory, SyscallStop, Tracer};
use crate::utils::short_temp_files::ShortTempFile;
use crate::{Context, R};
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use test_result::{TestResult, TestResults};

#[derive(Debug)]
pub struct SyscallMock {
    context: Context,
    tracee_pid: Pid,
    protocol: Protocol,
    result: TestResult,
    temporary_executables: Vec<ShortTempFile>,
}

impl SyscallMock {
    pub fn new(context: Context, tracee_pid: Pid, protocol: Protocol) -> SyscallMock {
        SyscallMock {
            context,
            tracee_pid,
            protocol,
            result: TestResult::Pass,
            temporary_executables: vec![],
        }
    }

    pub fn handle_syscall(
        &mut self,
        pid: Pid,
        syscall_stop: SyscallStop,
        syscall: Syscall,
        registers: user_regs_struct,
    ) -> R<()> {
        if let (Syscall::Execve, SyscallStop::Enter) = (&syscall, syscall_stop) {
            if self.tracee_pid != pid {
                let executable = tracee_memory::peek_string(pid, registers.rdi)?;
                let arguments = tracee_memory::peek_string_array(pid, registers.rsi)?;
                let mock_executable_path = self.handle_step(protocol::Command {
                    executable,
                    arguments,
                })?;
                tracee_memory::poke_string(
                    pid,
                    registers.rdi,
                    &mock_executable_path.as_os_str().as_bytes(),
                )?;
            }
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

    pub fn handle_end(&mut self, exitcode: i32) {
        if let Some(expected_step) = self.protocol.steps.pop_front() {
            self.register_error(&expected_step.command.format(), "<script terminated>");
        }
        if exitcode != 0 {
            self.register_error("<exitcode 0>", &format!("<exitcode {}>", exitcode));
        }
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

pub fn run_against_protocol(
    context: Context,
    executable: &Path,
    expected: Protocol,
) -> R<TestResult> {
    let syscall_mock = Tracer::run_against_mock(
        executable,
        expected.arguments.clone(),
        expected.env.clone(),
        |tracee_pid| SyscallMock::new(context, tracee_pid, expected),
    )?;
    Ok(syscall_mock.result)
}

#[cfg(test)]
mod run_against_protocol {
    use super::*;
    use crate::protocol::command::Command;
    use std::fs;
    use test_utils::{assert_error, trim_margin, TempFile};

    #[test]
    fn works_for_longer_file_names() -> R<()> {
        let long_command = TempFile::new()?;
        fs::copy("/bin/true", long_command.path())?;
        let script = TempFile::write_temp_script(
            trim_margin(&format!(
                r##"
                    |#!/usr/bin/env bash
                    |{}
                "##,
                long_command.path().to_string_lossy()
            ))?
            .as_bytes(),
        )?;
        assert_eq!(
            run_against_protocol(
                Context::new_test_context(),
                &script.path(),
                Protocol::new(vec![protocol::Step {
                    command: Command {
                        executable: long_command.path().as_os_str().as_bytes().to_vec(),
                        arguments: vec![],
                    },
                    stdout: vec![],
                    exitcode: 0
                }])
            )?,
            TestResult::Pass
        );
        Ok(())
    }

    #[test]
    fn complains_when_the_file_does_not_exist() {
        assert_error!(
            run_against_protocol(
                Context::new_test_context(),
                Path::new("./does_not_exist"),
                Protocol::empty()
            ),
            "ENOENT: No such file or directory"
        );
    }

    #[test]
    fn does_not_execute_the_commands() -> R<()> {
        let testfile = TempFile::new()?;
        let script = TempFile::write_temp_script(
            trim_margin(&format!(
                r##"
                    |#!/usr/bin/env bash
                    |touch {}
                "##,
                testfile.path().to_string_lossy()
            ))?
            .as_bytes(),
        )?;
        let _ = run_against_protocol(
            Context::new_test_context(),
            &script.path(),
            Protocol::empty(),
        )?;
        assert!(!testfile.path().exists(), "touch was executed");
        Ok(())
    }
}

pub fn run_against_protocols(
    context: Context,
    executable: &Path,
    expected: Vec<Protocol>,
) -> R<TestResults> {
    Ok(TestResults(
        expected
            .into_iter()
            .map(|expected| run_against_protocol(context.clone(), executable, expected))
            .collect::<R<Vec<TestResult>>>()?,
    ))
}
