#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]

mod emulation;
pub mod executable_mock;
mod protocol;
mod short_temp_files;
mod syscall_mocking;
mod tracee_memory;
pub mod utils;

use crate::emulation::run_against_protocol;
use crate::executable_mock::ExecutableMock;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

pub type R<A> = Result<A, Box<std::error::Error>>;

pub fn run_main(
    executable_mock: ExecutableMock,
    mut args: impl Iterator<Item = String>,
    stdout_handle: &mut impl Write,
) -> R<()> {
    let this_executable = args
        .next()
        .expect("argv: expected program name as argument 0");
    match args.next() {
        Some(argument) => {
            if argument == "--executable-mock" {
                ExecutableMock::run(vec![this_executable].into_iter().chain(args), stdout_handle)?;
            } else {
                write!(
                    stdout_handle,
                    "{}",
                    run_check_protocols(executable_mock, &PathBuf::from(argument))?
                )?;
            }
        }
        None => {
            Err("supply one argument")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod run_main {
    use super::*;
    use crate::executable_mock::ExecutableMock;
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
        run_main(ExecutableMock::get_test_mock(), args, &mut cursor)?;
        assert_eq!(String::from_utf8(cursor.into_inner())?, "second line\n");
        Ok(())
    }
}

pub fn run_check_protocols(executable_mock: ExecutableMock, script: &Path) -> R<String> {
    let expected = protocol::load(script)?;
    let errors = run_against_protocol(executable_mock, script, expected)?;
    Ok(match errors {
        None => "All tests passed.\n".to_string(),
        Some(error) => error,
    })
}
