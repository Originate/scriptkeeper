use crate::context::Context;
use crate::tracer::tracee_memory;
use crate::utils::short_temp_files::ShortTempFile;
use crate::{ExitCode, R};
use bincode::{deserialize, serialize};
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub stdout: Vec<u8>,
    pub exitcode: i32,
}

#[derive(Debug)]
pub struct ExecutableMock {
    temp_file: ShortTempFile,
}

impl ExecutableMock {
    pub fn new(context: &Context, mock_config: Config) -> R<ExecutableMock> {
        let mut contents = b"#!".to_vec();
        contents.append(
            &mut context
                .scriptkeeper_executable()
                .as_os_str()
                .as_bytes()
                .to_vec(),
        );
        contents.append(&mut b" --executable-mock\n".to_vec());
        contents.append(&mut serialize(&mock_config)?);
        let temp_file = ShortTempFile::new(&contents)?;
        Ok(ExecutableMock { temp_file })
    }

    pub fn path(&self) -> PathBuf {
        self.temp_file.path()
    }

    pub fn poke_for_execve_syscall(
        pid: Pid,
        registers: &user_regs_struct,
        executable_mock_path: PathBuf,
    ) -> R<()> {
        tracee_memory::poke_single_word_string(
            pid,
            registers.rdi,
            &executable_mock_path.as_os_str().as_bytes(),
        )
    }

    pub fn run(context: &Context, executable_mock_path: &Path) -> R<ExitCode> {
        let config: Config = deserialize(&ExecutableMock::skip_hashbang_line(fs::read(
            executable_mock_path,
        )?))?;
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
}

#[cfg(test)]
mod test {
    use super::*;
    use std::process::Command;
    use test_utils::TempFile;

    #[test]
    fn creates_an_executable_that_outputs_the_given_stdout() -> R<()> {
        let mock_executable = ExecutableMock::new(
            &Context::new_mock(),
            Config {
                stdout: b"foo".to_vec(),
                exitcode: 0,
            },
        )?;
        let output = Command::new(mock_executable.path()).output()?;
        assert_eq!(output.stdout, b"foo");
        Ok(())
    }

    #[test]
    fn creates_an_executable_that_exits_with_the_given_exitcode() -> R<()> {
        let mock_executable = ExecutableMock::new(
            &Context::new_mock(),
            Config {
                stdout: b"foo".to_vec(),
                exitcode: 42,
            },
        )?;
        let output = Command::new(mock_executable.path()).output()?;
        assert_eq!(output.status.code(), Some(42));
        Ok(())
    }
}
