#![cfg_attr(feature = "dev", allow(dead_code, unused_variables))]
#![cfg_attr(feature = "ci", deny(warnings))]

mod syscall_mocking;
mod tracee_memory;

use crate::syscall_mocking::{Syscall, SyscallStop, Tracer};
use crate::tracee_memory::{data_to_string, ptrace_peekdata_iter, ptrace_pokedata, string_to_data};
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::fs::copy;
use std::path::{Path, PathBuf};

pub type R<A> = Result<A, Box<std::error::Error>>;

#[derive(Debug)]
pub struct SyscallMock {
    script_pid: Pid,
    execve_paths: Vec<PathBuf>,
}

impl SyscallMock {
    fn new(script_pid: Pid) -> SyscallMock {
        SyscallMock {
            script_pid,
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
            if self.script_pid != pid {
                let path = data_to_string(ptrace_peekdata_iter(pid, registers.rdi))?;
                copy("/bin/true", "/tmp/a")?;
                ptrace_pokedata(pid, registers.rdi, string_to_data("/tmp/a")?)?;
                self.execve_paths.push(PathBuf::from(path.clone()));
            }
        }
        Ok(())
    }
}

pub fn execve_paths(executable: &Path) -> R<Vec<PathBuf>> {
    Ok(Tracer::run_against_mock(executable)?.execve_paths)
}

#[cfg(test)]
mod test_execve_paths {
    use super::*;
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
            execve_paths(&script.path())?.first().unwrap(),
            &PathBuf::from("./true")
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
            execve_paths(&script.path())?,
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
        assert_eq!(execve_paths(&script.path())?, vec![long_command.path()]);
        Ok(())
    }

    #[test]
    fn complains_when_the_file_does_not_exist() {
        assert_eq!(
            format!(
                "{}",
                execve_paths(Path::new("./does_not_exist")).unwrap_err()
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
        execve_paths(&script.path())?;
        assert!(!testfile.path().exists(), "touch was executed");
        Ok(())
    }
}

pub fn run(script: &Path) -> R<String> {
    Ok(format!("executables: {:?}\n", execve_paths(script)?))
}
