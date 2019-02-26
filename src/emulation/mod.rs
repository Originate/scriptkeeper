pub mod executable_mock;
pub mod test_result;

use crate::protocol;
use crate::protocol::Protocol;
use crate::syscall_mocking::syscall::Syscall;
use crate::syscall_mocking::{tracee_memory, SyscallStop, Tracer};
use crate::utils::path_to_string;
use crate::utils::short_temp_files::ShortTempFile;
use crate::{Context, R};
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::path::{Path, PathBuf};
use test_result::{TestResult, TestResults};

#[derive(Debug)]
pub struct SyscallMock {
    context: Context,
    tracee_pid: Pid,
    expected: Protocol,
    result: TestResult,
    temporary_executables: Vec<ShortTempFile>,
}

impl SyscallMock {
    pub fn new(context: Context, tracee_pid: Pid, expected: Protocol) -> SyscallMock {
        SyscallMock {
            context,
            tracee_pid,
            expected,
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
                let executable = tracee_memory::data_to_string(tracee_memory::peekdata_iter(
                    pid,
                    registers.rdi,
                ))?;
                let arguments = tracee_memory::peek_string_array(pid, registers.rsi)?;
                let mock_executable_path = self.handle_step(protocol::Command {
                    executable,
                    arguments,
                })?;
                tracee_memory::pokedata(
                    pid,
                    registers.rdi,
                    tracee_memory::string_to_data(path_to_string(&mock_executable_path)?)?,
                )?;
            }
        }
        Ok(())
    }

    fn handle_step(&mut self, received: protocol::Command) -> R<PathBuf> {
        let stdout = match self.expected.steps.pop_front() {
            Some(next_expected_step) => {
                if next_expected_step.command != received {
                    self.register_error(&next_expected_step.command.format(), &received.format());
                }
                next_expected_step.stdout
            }
            None => {
                self.register_error("<protocol end>", &received.format());
                vec![]
            }
        };
        let mock_executable_contents =
            executable_mock::create_mock_executable(&self.context, stdout);
        let temp_executable = ShortTempFile::new(&mock_executable_contents)?;
        let path = temp_executable.path();
        self.temporary_executables.push(temp_executable);
        Ok(path)
    }

    pub fn handle_end(&mut self, exitcode: i32) {
        if let Some(expected_step) = self.expected.steps.pop_front() {
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
                        executable: long_command.path().to_string_lossy().into_owned(),
                        arguments: vec![],
                    },
                    stdout: vec![]
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
