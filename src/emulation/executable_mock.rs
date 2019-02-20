use crate::{Context, R};
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub fn create_mock_executable(context: &Context, mut stdout: Vec<u8>) -> Vec<u8> {
    let mut result = b"#!".to_vec();
    result.append(
        &mut context
            .check_protocols_executable
            .as_os_str()
            .as_bytes()
            .to_vec(),
    );
    result.append(&mut b" --executable-mock\n".to_vec());
    result.append(&mut stdout);
    result
}

pub fn run(executable_mock_path: &Path, stdout_handle: &mut impl Write) -> R<()> {
    let output = skip_shabang_line(fs::read(executable_mock_path)?);
    stdout_handle.write_all(&output)?;
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

    mod run {
        use super::*;
        use std::io::Cursor;
        use test_utils::TempFile;

        #[test]
        fn outputs_the_argument_file_while_skipping_the_first_line() -> R<()> {
            let file = TempFile::write_temp_script("first line\nsecond line\n")?;
            let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![]);
            run(&file.path(), &mut cursor)?;
            assert_eq!(String::from_utf8(cursor.into_inner())?, "second line\n");
            Ok(())
        }
    }

    #[test]
    fn renders_an_executable_that_outputs_the_given_stdout() -> R<()> {
        let mock_executable = TempFile::write_temp_script(&String::from_utf8(
            create_mock_executable(&Context::new_test_context(), b"foo".to_vec()),
        )?)?;
        let output = Command::new(mock_executable.path()).output()?;
        assert_eq!(output.stdout, b"foo");
        Ok(())
    }
}
