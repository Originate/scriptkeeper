#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]

mod emulation;
mod executable_mock;
mod protocol;
mod syscall_mocking;
mod tracee_memory;
mod utils;

use crate::emulation::run_against_protocol;
use std::io::Write;
use std::path::{Path, PathBuf};

pub type R<A> = Result<A, Box<std::error::Error>>;

#[derive(Debug)]
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

enum Args {
    ExecutableMock { executable_mock_path: PathBuf },
    CheckProtocols { script_path: PathBuf },
}

fn parse_args(mut args: impl Iterator<Item = String>) -> R<Args> {
    args.next()
        .ok_or("argv: expected program name as argument 0")?;
    Ok(match args.next().ok_or("supply one argument")?.as_ref() {
        "--executable-mock" => Args::ExecutableMock {
            executable_mock_path: PathBuf::from(
                args.next().expect("expected executable file as argument 1"),
            ),
        },
        argument => Args::CheckProtocols {
            script_path: PathBuf::from(argument),
        },
    })
}

pub fn run_main(
    context: Context,
    args: impl Iterator<Item = String>,
    stdout_handle: &mut impl Write,
) -> R<()> {
    match parse_args(args)? {
        Args::ExecutableMock {
            executable_mock_path,
        } => executable_mock::run(&executable_mock_path, stdout_handle)?,
        Args::CheckProtocols { script_path } => write!(
            stdout_handle,
            "{}",
            run_check_protocols(context, &script_path)?
        )?,
    }
    Ok(())
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

pub fn run_check_protocols(context: Context, script: &Path) -> R<String> {
    let expected = protocol::load(script)?;
    let errors = run_against_protocol(context, script, expected)?;
    Ok(match errors {
        None => "All tests passed.\n".to_string(),
        Some(error) => error,
    })
}
