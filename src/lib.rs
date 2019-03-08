#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]
#![allow(clippy::needless_range_loop)]
#![cfg_attr(test, allow(clippy::module_inception))]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate memoffset;

mod cli;
mod protocol;
mod syscall_mock;
mod tracer;
pub mod utils;

use crate::protocol::{Protocol, Protocols};
use crate::syscall_mock::test_result::{TestResult, TestResults};
use crate::syscall_mock::{executable_mock, SyscallMock};
use crate::tracer::Tracer;
use std::io::Write;
use std::path::{Path, PathBuf};

pub type R<A> = Result<A, Box<std::error::Error>>;

#[derive(Debug, PartialEq)]
pub struct ExitCode(pub i32);

impl ExitCode {
    pub fn exit(self) {
        std::process::exit(self.0);
    }
}

pub fn wrap_main<F: FnOnce(ExitCode)>(exit: F, main: fn() -> R<ExitCode>) {
    match main() {
        Ok(exitcode) => exit(exitcode),
        Err(err) => {
            eprintln!("error: {}", err.description());
            exit(ExitCode(1));
        }
    };
}

#[cfg(test)]
mod wrap_main {
    use super::*;

    #[test]
    fn calls_exit_when_given_a_non_zero_exit_code() -> R<()> {
        let mut exitcodes = vec![];
        let mock_exit = |exitcode| exitcodes.push(exitcode);
        let main = || Ok(ExitCode(1));
        wrap_main(mock_exit, main);
        assert_eq!(exitcodes, vec![ExitCode(1)]);
        Ok(())
    }

    #[test]
    fn calls_exit_with_a_non_zero_exit_code_when_the_given_main_function_fails() -> R<()> {
        let mut exitcodes = vec![];
        let mock_exit = |exitcode| exitcodes.push(exitcode);
        let main = || Err("foo")?;
        wrap_main(mock_exit, main);
        assert_eq!(exitcodes, vec![ExitCode(1)]);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    check_protocols_executable: PathBuf,
}

impl Context {
    pub fn new() -> R<Context> {
        Ok(Context {
            check_protocols_executable: std::env::current_exe()?,
        })
    }

    pub fn new_test_context() -> Context {
        let cwd = std::env::current_dir().unwrap();
        Context {
            check_protocols_executable: cwd.join("./target/debug/check-protocols"),
        }
    }
}

pub fn run_main(
    context: Context,
    args: impl Iterator<Item = String>,
    stdout_handle: &mut impl Write,
) -> R<ExitCode> {
    Ok(match cli::parse_args(args)? {
        cli::Args::ExecutableMock {
            executable_mock_path,
        } => executable_mock::run(&executable_mock_path, stdout_handle)?,
        cli::Args::CheckProtocols { script_path } => {
            let (exitcode, output) = run_check_protocols(context, &script_path)?;
            write!(stdout_handle, "{}", output)?;
            exitcode
        }
    })
}

#[cfg(test)]
mod run_main {
    use super::*;
    use executable_mock::create_mock_executable;
    use std::io::Cursor;
    use test_utils::TempFile;

    #[test]
    fn when_passed_executable_mock_flag_behaves_like_executable_mock() -> R<()> {
        let context = Context::new_test_context();
        let executable_contents = create_mock_executable(
            &context,
            executable_mock::Config {
                stdout: b"foo".to_vec(),
                exitcode: 0,
            },
        )?;
        let file = TempFile::write_temp_script(&executable_contents)?;
        let args = vec![
            "executable-mock".to_string(),
            "--executable-mock".to_string(),
            file.path().to_string_lossy().into_owned(),
        ]
        .into_iter();
        let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![]);
        run_main(context, args, &mut cursor)?;
        assert_eq!(cursor.into_inner(), b"foo");
        Ok(())
    }
}

pub fn run_check_protocols(context: Context, script: &Path) -> R<(ExitCode, String)> {
    if !script.exists() {
        Err(format!(
            "executable file not found: {}",
            script.to_string_lossy()
        ))?
    }
    let expected_protocols = Protocols::load(script)?;
    let results = run_against_protocols(context, script, expected_protocols)?;
    Ok((results.exitcode(), results.format_test_results()))
}

fn run_against_protocols(
    context: Context,
    program: &Path,
    Protocols {
        protocols,
        unmocked_commands,
        interpreter,
    }: Protocols,
) -> R<TestResults> {
    Ok(TestResults(
        protocols
            .into_iter()
            .map(|expected| {
                run_against_protocol(
                    context.clone(),
                    &interpreter,
                    program,
                    expected,
                    &unmocked_commands,
                )
            })
            .collect::<R<Vec<TestResult>>>()?,
    ))
}

pub fn run_against_protocol(
    context: Context,
    interpreter: &Option<Vec<u8>>,
    program: &Path,
    expected: Protocol,
    unmocked_commands: &[Vec<u8>],
) -> R<TestResult> {
    let syscall_mock = Tracer::run_against_mock(
        interpreter,
        program,
        expected.arguments.clone(),
        expected.env.clone(),
        |tracee_pid| SyscallMock::new(context, tracee_pid, expected, unmocked_commands),
    )?;
    Ok(syscall_mock.result)
}

#[cfg(test)]
mod run_against_protocol {
    use super::*;
    use crate::protocol::command::Command;
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use test_utils::{trim_margin, TempFile};

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
                &None,
                &script.path(),
                Protocol::new(vec![protocol::Step {
                    command: Command {
                        executable: long_command.path().as_os_str().as_bytes().to_vec(),
                        arguments: vec![],
                    },
                    stdout: vec![],
                    exitcode: 0
                }]),
                &[]
            )?,
            TestResult::Pass
        );
        Ok(())
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
            &None,
            &script.path(),
            Protocol::empty(),
            &[],
        )?;
        assert!(!testfile.path().exists(), "touch was executed");
        Ok(())
    }
}
