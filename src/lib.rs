#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]

mod syscall_mocking;
mod tracee_memory;

use crate::syscall_mocking::{Syscall, SyscallStop, Tracer};
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::fs::copy;
use std::path::{Path, PathBuf};

pub type R<A> = Result<A, Box<std::error::Error>>;

#[derive(Debug)]
pub struct SyscallMock {
    tracee_pid: Pid,
    execve_paths: Vec<(PathBuf, Vec<String>)>,
}

impl SyscallMock {
    fn new(tracee_pid: Pid) -> SyscallMock {
        SyscallMock {
            tracee_pid,
            execve_paths: vec![],
        }
    }

    fn handle_syscall(
        &mut self,
        pid: Pid,
        syscall_stop: SyscallStop,
        syscall: Syscall,
        registers: user_regs_struct,
    ) -> R<()> {
        if let (Syscall::Execve, SyscallStop::Enter) = (&syscall, syscall_stop) {
            if self.tracee_pid != pid {
                let path = tracee_memory::data_to_string(tracee_memory::peekdata_iter(
                    pid,
                    registers.rdi,
                ))?;
                copy("/bin/true", "/tmp/a")?;
                tracee_memory::pokedata(
                    pid,
                    registers.rdi,
                    tracee_memory::string_to_data("/tmp/a")?,
                )?;
                self.execve_paths.push((
                    PathBuf::from(path),
                    tracee_memory::peek_string_array(pid, registers.rsi)?,
                ));
            }
        }
        Ok(())
    }
}

pub fn emulate_executable(executable: &Path) -> R<Vec<(PathBuf, Vec<String>)>> {
    Ok(Tracer::run_against_mock(executable)?.execve_paths)
}

#[cfg(test)]
mod test_emulate_executable {
    extern crate map_in_place;

    use super::*;
    use map_in_place::MapVecInPlace;
    use std::fs;
    use std::process::Command;
    use tempdir::TempDir;

    struct TempFile {
        tempdir: TempDir,
    }

    impl TempFile {
        fn new() -> R<TempFile> {
            let tempdir = TempDir::new("test")?;
            Ok(TempFile { tempdir })
        }

        fn path(&self) -> PathBuf {
            self.tempdir.path().join("file")
        }
    }

    fn run(command: &str, args: Vec<&str>) -> R<()> {
        let status = Command::new(command).args(args).status()?;
        if status.success() {
            Ok(())
        } else {
            Err("command failed")?
        }
    }

    fn write_temp_script(script: &str) -> R<TempFile> {
        let tempfile = TempFile::new()?;
        fs::write(&tempfile.path(), script.trim_start())?;
        run("chmod", vec!["+x", tempfile.path().to_str().unwrap()])?;
        Ok(tempfile)
    }

    #[test]
    fn returns_the_path_of_the_first_executable_spawned_by_the_script() -> R<()> {
        let script = write_temp_script(
            r##"
                #!/usr/bin/env bash

                cd /bin
                ./true
            "##,
        )?;
        assert_eq!(
            emulate_executable(&script.path())?.first().unwrap().0,
            PathBuf::from("./true")
        );
        Ok(())
    }

    #[test]
    fn returns_multiple_executables_spawned_by_the_script() -> R<()> {
        let script = write_temp_script(
            r##"
                #!/usr/bin/env bash

                cd /bin
                ./true
                ./false
            "##,
        )?;
        assert_eq!(
            emulate_executable(&script.path())?.map(|x| x.0),
            vec![PathBuf::from("./true"), PathBuf::from("./false")]
        );
        Ok(())
    }

    #[test]
    fn works_for_longer_file_names() -> R<()> {
        let long_command = TempFile::new()?;
        copy("/bin/true", long_command.path())?;
        let script = write_temp_script(&format!(
            r##"
                #!/usr/bin/env bash

                {}
            "##,
            long_command.path().to_str().unwrap()
        ))?;
        assert_eq!(
            emulate_executable(&script.path())?.map(|x| x.0),
            vec![long_command.path()]
        );
        Ok(())
    }

    #[test]
    fn complains_when_the_file_does_not_exist() {
        assert_eq!(
            format!(
                "{}",
                emulate_executable(Path::new("./does_not_exist")).unwrap_err()
            ),
            "ENOENT: No such file or directory"
        );
    }

    #[test]
    fn does_not_execute_the_commands() -> R<()> {
        let testfile = TempFile::new()?;
        let script = write_temp_script(&format!(
            r##"
                #!/usr/bin/env bash

                touch {}
            "##,
            testfile.path().to_str().ok_or("utf8 error")?
        ))?;
        emulate_executable(&script.path())?;
        assert!(!testfile.path().exists(), "touch was executed");
        Ok(())
    }
}

pub fn run(script: &Path) -> R<String> {
    Ok(format!("executables: {:?}\n", emulate_executable(script)?))
}
