#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]
#![cfg_attr(test, allow(clippy::module_inception))]

mod cli;
mod emulation;
mod protocol;
mod syscall_mocking;
pub mod utils;

use crate::emulation::{executable_mock, run_against_protocols};
use protocol::Protocol;
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
        Context {
            check_protocols_executable: PathBuf::from("./target/debug/check-protocols"),
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
        } => {
            executable_mock::run(&executable_mock_path, stdout_handle)?;
            ExitCode(0)
        }
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
    use std::io::Cursor;
    use test_utils::TempFile;

    #[test]
    fn when_passed_executable_mock_flag_behaves_like_executable_mock() -> R<()> {
        let file = TempFile::write_temp_script("first line\nsecond line\n")?;
        let args = vec![
            "executable-mock".to_string(),
            "--executable-mock".to_string(),
            file.path().to_string_lossy().into_owned(),
        ]
        .into_iter();
        let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![]);
        run_main(Context::new_test_context(), args, &mut cursor)?;
        assert_eq!(String::from_utf8(cursor.into_inner())?, "second line\n");
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
    let expected_protocols = Protocol::load(script)?;
    let results = run_against_protocols(context, script, expected_protocols)?;
    Ok((results.exitcode(), results.format_test_results()))
}
