use crate::{Context, R};
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

pub fn render_mock_executable(context: &Context, mut stdout: Vec<u8>) -> Vec<u8> {
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

pub fn run(mut args: impl Iterator<Item = String>, stdout_handle: &mut impl Write) -> R<()> {
    args.next()
        .expect("argv: expected program name as argument 0");
    let file = args.next().expect("expected executable file as argument 1");
    let output = skip_shabang_line(fs::read(&file)?);
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
            let args = vec![
                "executable-mock".to_string(),
                file.path().to_string_lossy().into_owned(),
            ]
            .into_iter();
            let mut cursor: Cursor<Vec<u8>> = Cursor::new(vec![]);
            run(args, &mut cursor)?;
            assert_eq!(String::from_utf8(cursor.into_inner())?, "second line\n");
            Ok(())
        }
    }

    #[test]
    fn renders_an_executable_that_outputs_the_given_stdout() -> R<()> {
        let mock_executable = TempFile::write_temp_script(&String::from_utf8(
            render_mock_executable(&Context::new_test_context(), b"foo".to_vec()),
        )?)?;
        let output = Command::new(mock_executable.path()).output()?;
        assert_eq!(output.stdout, b"foo");
        Ok(())
    }
}
