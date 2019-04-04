use crate::context::Context;
use crate::{ExitCode, R};
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
    if !context.scriptkeeper_executable().exists() {
        Err(format!(
            "scriptkeeper bug: scriptkeeper_executable can't be found: {}",
            context.scriptkeeper_executable().to_string_lossy()
        ))?;
    }
    let mut result = b"#!".to_vec();
    result.append(
        &mut context
            .scriptkeeper_executable()
            .as_os_str()
            .as_bytes()
            .to_vec(),
    );
    result.append(&mut b" --executable-mock\n".to_vec());
    result.append(&mut serialize(&config)?);
    Ok(result)
}

pub fn run(context: &Context, executable_mock_path: &Path) -> R<ExitCode> {
    let config: Config = deserialize(&skip_hashbang_line(fs::read(executable_mock_path)?))?;
    context.stdout().write_all(&config.stdout)?;
    Ok(ExitCode(config.exitcode))
}

fn skip_hashbang_line(input: Vec<u8>) -> Vec<u8> {
    input
        .clone()
        .into_iter()
        .skip_while(|char: &u8| *char != b'\n')
        .skip(1)
        .collect()
}

#[cfg(test)]
mod create_mock_executable {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;
    use test_utils::{assert_error, TempFile};

    #[test]
    fn renders_an_executable_that_outputs_the_given_stdout() -> R<()> {
        let mock_executable = TempFile::write_temp_script(&create_mock_executable(
            &Context::new_mock(),
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
            &Context::new_mock(),
            Config {
                stdout: b"foo".to_vec(),
                exitcode: 42,
            },
        )?)?;
        let output = Command::new(mock_executable.path()).output()?;
        assert_eq!(output.status.code(), Some(42));
        Ok(())
    }

    #[test]
    fn aborts_with_a_helpful_message_when_scriptkeeper_executable_does_not_exist() {
        let context = Context::Context {
            scriptkeeper_executable: PathBuf::from("/bin/does_not_exist"),
        };
        assert_error!(
            create_mock_executable(
                &context,
                Config {
                    stdout: vec![],
                    exitcode: 42,
                },
            ),
            "scriptkeeper bug: scriptkeeper_executable can't be found: /bin/does_not_exist"
        );
    }
}
