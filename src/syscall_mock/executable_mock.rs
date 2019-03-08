use crate::{Context, ExitCode, R};
use bincode::{deserialize, serialize};
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub stdout: Vec<u8>,
    pub exitcode: i32,
}

pub fn create_mock_executable(context: &Context, config: Config) -> R<Vec<u8>> {
    let mut result = b"#!".to_vec();
    result.append(
        &mut context
            .check_protocols_executable
            .as_os_str()
            .as_bytes()
            .to_vec(),
    );
    result.append(&mut b" --executable-mock\n".to_vec());
    result.append(&mut serialize(&config)?);
    Ok(result)
}

pub fn run(executable_mock_path: &Path, stdout_handle: &mut impl Write) -> R<ExitCode> {
    let config: Config = deserialize(&skip_hashbang_line(fs::read(executable_mock_path)?))?;
    stdout_handle.write_all(&config.stdout)?;
    Ok(ExitCode(config.exitcode))
}

const NEWLINE: u8 = 0x0A;

fn skip_hashbang_line(input: Vec<u8>) -> Vec<u8> {
    input
        .clone()
        .into_iter()
        .skip_while(|char: &u8| *char != NEWLINE)
        .skip(1)
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::process::Command;
    use test_utils::TempFile;

    #[test]
    fn renders_an_executable_that_outputs_the_given_stdout() -> R<()> {
        let mock_executable = TempFile::write_temp_script(&create_mock_executable(
            &Context::new_test_context(),
            Config {
                stdout: b"foo".to_vec(),
                exitcode: 0,
            },
        )?)?;
        let output = Command::new(mock_executable.path()).output()?;
        assert_eq!(output.stdout, b"foo");
        Ok(())
    }

    #[test]
    fn renders_an_executable_that_exits_with_the_given_exitcode() -> R<()> {
        let mock_executable = TempFile::write_temp_script(&create_mock_executable(
            &Context::new_test_context(),
            Config {
                stdout: b"foo".to_vec(),
                exitcode: 42,
            },
        )?)?;
        let output = Command::new(mock_executable.path()).output()?;
        assert_eq!(output.status.code(), Some(42));
        Ok(())
    }
}
