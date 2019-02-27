use crate::{Context, R};
use bincode::{deserialize, serialize};
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct MockConfig {
    stdout: Vec<u8>,
}

pub fn create_mock_executable(context: &Context, stdout: Vec<u8>) -> R<Vec<u8>> {
    let mut result = b"#!".to_vec();
    result.append(
        &mut context
            .check_protocols_executable
            .as_os_str()
            .as_bytes()
            .to_vec(),
    );
    result.append(&mut b" --executable-mock\n".to_vec());
    result.append(&mut serialize(&MockConfig { stdout })?);
    Ok(result)
}

pub fn run(executable_mock_path: &Path, stdout_handle: &mut impl Write) -> R<()> {
    let output: MockConfig = deserialize(&skip_shabang_line(fs::read(executable_mock_path)?))?;
    stdout_handle.write_all(&output.stdout)?;
    Ok(())
}

const NEWLINE: u8 = 0x0A;

fn skip_shabang_line(input: Vec<u8>) -> Vec<u8> {
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
            b"foo".to_vec(),
        )?)?;
        let output = Command::new(mock_executable.path()).output()?;
        assert_eq!(output.stdout, b"foo");
        Ok(())
    }
}
