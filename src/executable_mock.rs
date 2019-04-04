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
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub enum Config {
    Config { stdout: Vec<u8>, exitcode: i32 },
    Wrapper { executable: PathBuf },
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

    fn wrapper(context: &Context, executable: &Path) -> R<ExecutableMock> {
        ExecutableMock::new(
            context,
            Config::Wrapper {
                executable: executable.to_owned(),
            },
        )
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
        match config {
            Config::Config { stdout, exitcode } => {
                context.stdout().write_all(&stdout)?;
                Ok(ExitCode(exitcode))
            }
            Config::Wrapper { executable } => {
                Command::new(&executable).output()?;
                Ok(ExitCode(0))
            }
        }
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
    use test_utils::{trim_margin, TempFile};

    mod new {
        use super::*;

        #[test]
        fn creates_an_executable_that_outputs_the_given_stdout() -> R<()> {
            let executable_mock = ExecutableMock::new(
                &Context::new_mock(),
                Config::Config {
                    stdout: b"foo".to_vec(),
                    exitcode: 0,
                },
            )?;
            let output = Command::new(&executable_mock.path()).output();
            assert_eq!(output?.stdout, b"foo");
            Ok(())
        }

        #[test]
        fn creates_an_executable_that_exits_with_the_given_exitcode() -> R<()> {
            let executable_mock = ExecutableMock::new(
                &Context::new_mock(),
                Config::Config {
                    stdout: b"foo".to_vec(),
                    exitcode: 42,
                },
            )?;
            let output = Command::new(executable_mock.path()).output()?;
            assert_eq!(output.status.code(), Some(42));
            Ok(())
        }
    }

    mod wrapper {
        use super::*;
        use crate::utils::path_to_string;
        use tempdir::TempDir;

        #[test]
        fn executes_the_given_command() -> R<()> {
            let temp_dir = TempDir::new("test")?;
            let path = temp_dir.path().join("foo.txt");
            let script = TempFile::write_temp_script(
                trim_margin(&format!(
                    "
                        |#!/usr/bin/env bash
                        |echo foo > {}
                    ",
                    path_to_string(&path)?
                ))?
                .as_bytes(),
            )?;
            let executable_mock = ExecutableMock::wrapper(&Context::new_mock(), &script.path())?;
            Command::new(executable_mock.path()).status()?;
            assert_eq!(String::from_utf8(fs::read(&path)?)?, "foo\n");
            Ok(())
        }
    }
}
