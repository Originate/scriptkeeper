use crate::executable_mock;
use crate::protocol;
use crate::protocol::Protocol;
use crate::syscall_mocking::{Syscall, SyscallStop, Tracer};
use crate::tracee_memory;
use crate::utils::path_to_string;
use crate::utils::short_temp_files::ShortTempFile;
use crate::{Context, R};
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct SyscallMock {
    context: Context,
    tracee_pid: Pid,
    expected: Protocol,
    errors: Option<String>,
    temporary_executables: Vec<ShortTempFile>,
}

impl SyscallMock {
    pub fn new(context: Context, tracee_pid: Pid, expected: Protocol) -> SyscallMock {
        SyscallMock {
            context,
            tracee_pid,
            expected,
            errors: None,
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
                let command = tracee_memory::data_to_string(tracee_memory::peekdata_iter(
                    pid,
                    registers.rdi,
                ))?;
                let arguments = tracee_memory::peek_string_array(pid, registers.rsi)?;
                let mock_executable_path = self.handle_step(&command, arguments)?;
                tracee_memory::pokedata(
                    pid,
                    registers.rdi,
                    tracee_memory::string_to_data(path_to_string(&mock_executable_path)?)?,
                )?;
            }
        }
        Ok(())
    }

    fn handle_step(
        &mut self,
        received_command: &str,
        received_arguments: Vec<String>,
    ) -> R<PathBuf> {
        let stdout = match self.expected.pop_front() {
            Some(next_expected_step) => {
                match next_expected_step.compare(received_command, received_arguments) {
                    Ok(()) => {}
                    Err(error) => self.push_error(error),
                }
                next_expected_step.stdout
            }
            None => {
                self.push_error(protocol::Step::format_error(
                    "<protocol end>",
                    &protocol::format_command(&received_command, received_arguments),
                ));
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

    fn handle_end(&mut self) {
        if let Some(expected_step) = self.expected.pop_front() {
            self.push_error(protocol::Step::format_error(
                &protocol::format_command(&expected_step.command, expected_step.arguments),
                "<script terminated>",
            ));
        }
    }

    fn push_error(&mut self, error: String) {
        if self.errors.is_none() {
            self.errors = Some(error);
        }
    }
}

pub fn run_against_protocol(
    context: Context,
    executable: &Path,
    expected: Protocol,
) -> R<Option<String>> {
    let mut syscall_mock = Tracer::run_against_mock(executable, |tracee_pid| {
        SyscallMock::new(context, tracee_pid, expected)
    })?;
    syscall_mock.handle_end();
    Ok(syscall_mock.errors)
}

#[cfg(test)]
mod run_against_protocol {
    extern crate map_in_place;

    use super::*;
    use std::collections::vec_deque::VecDeque;
    use std::fs;
    use test_utils::TempFile;

    #[test]
    fn works_for_longer_file_names() -> R<()> {
        let long_command = TempFile::new()?;
        fs::copy("/bin/true", long_command.path())?;
        let script = TempFile::write_temp_script(&format!(
            r##"
                #!/usr/bin/env bash

                {}
            "##,
            long_command.path().to_string_lossy()
        ))?;
        assert_eq!(
            run_against_protocol(
                Context::new_test_context(),
                &script.path(),
                vec![protocol::Step {
                    command: long_command.path().to_string_lossy().into_owned(),
                    arguments: vec![],
                    stdout: vec![]
                }]
                .into()
            )?,
            None
        );
        Ok(())
    }

    #[test]
    fn complains_when_the_file_does_not_exist() {
        assert_eq!(
            format!(
                "{}",
                run_against_protocol(
                    Context::new_test_context(),
                    Path::new("./does_not_exist"),
                    VecDeque::new()
                )
                .unwrap_err()
            ),
            "ENOENT: No such file or directory"
        );
    }

    #[test]
    fn does_not_execute_the_commands() -> R<()> {
        let testfile = TempFile::new()?;
        let script = TempFile::write_temp_script(&format!(
            r##"
                #!/usr/bin/env bash

                touch {}
            "##,
            testfile.path().to_string_lossy()
        ))?;
        run_against_protocol(Context::new_test_context(), &script.path(), VecDeque::new())?;
        assert!(!testfile.path().exists(), "touch was executed");
        Ok(())
    }
}
