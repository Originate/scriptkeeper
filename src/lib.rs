#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]
#![allow(clippy::needless_range_loop, clippy::large_enum_variant)]
#![cfg_attr(test, allow(clippy::module_inception))]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate memoffset;

pub mod cli;
pub mod context;
mod recorder;
mod test_checker;
mod test_spec;
mod tracer;
pub mod utils;

use crate::context::Context;
use crate::recorder::{hole_recorder::run_against_tests, Recorder};
use crate::test_checker::executable_mock;
use crate::test_spec::yaml::write_yaml;
use crate::test_spec::Tests;
use crate::tracer::stdio_redirecting::CaptureStderr;
use crate::tracer::Tracer;
use std::collections::HashMap;
use std::path::Path;

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

pub fn run_main(context: &Context, args: &cli::Args) -> R<ExitCode> {
    Ok(match args {
        cli::Args::ExecutableMock {
            executable_mock_path,
        } => executable_mock::run(context, &executable_mock_path)?,
        cli::Args::Scriptkeeper {
            script_path,
            record,
        } => {
            if *record {
                print_recorded_test(context, script_path)?
            } else {
                run_scriptkeeper(context, &script_path)?
            }
        }
    })
}

#[cfg(test)]
mod run_main {
    use super::*;
    use executable_mock::create_mock_executable;
    use test_utils::TempFile;

    #[test]
    fn when_passed_executable_mock_flag_behaves_like_executable_mock() -> R<()> {
        let context = Context::new_mock();
        let executable_contents = create_mock_executable(
            &context,
            executable_mock::Config {
                stdout: b"foo".to_vec(),
                exitcode: 0,
            },
        )?;
        let executable_mock = TempFile::write_temp_script(&executable_contents)?;
        run_main(
            &context,
            &cli::Args::ExecutableMock {
                executable_mock_path: executable_mock.path(),
            },
        )?;
        assert_eq!(context.get_captured_stdout(), "foo");
        Ok(())
    }
}

pub fn run_scriptkeeper(context: &Context, script: &Path) -> R<ExitCode> {
    if !script.exists() {
        Err(format!(
            "executable file not found: {}",
            script.to_string_lossy()
        ))?
    }
    let (test_file_path, tests) = Tests::load(script)?;
    run_against_tests(&context, script, &test_file_path, tests)
}

fn print_recorded_test(context: &Context, program: &Path) -> R<ExitCode> {
    let test = Tracer::run_against_mock(
        context,
        &None,
        program,
        vec![],
        HashMap::new(),
        CaptureStderr::NoCapture,
        Recorder::empty(),
    )?;
    write_yaml(&mut *context.stdout(), &Tests::new(vec![test]).serialize()?)?;
    Ok(ExitCode(0))
}
