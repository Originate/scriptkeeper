use crate::protocol;
use crate::protocol::Protocol;
use crate::syscall_mocking::{Syscall, SyscallStop, Tracer};
use crate::tracee_memory;
use crate::R;
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::fs::copy;
use std::path::Path;

#[derive(Debug)]
pub struct SyscallMock {
    tracee_pid: Pid,
    execve_paths: Protocol,
}

impl SyscallMock {
    pub fn new(tracee_pid: Pid) -> SyscallMock {
        SyscallMock {
            tracee_pid,
            execve_paths: vec![],
        }
    }

    pub fn handle_syscall(
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
                self.execve_paths.push(protocol::Step {
                    command: path,
                    arguments: tracee_memory::peek_string_array(pid, registers.rsi)?,
                });
            }
        }
        Ok(())
    }
}

pub fn emulate_executable(executable: &Path) -> R<Protocol> {
    Ok(Tracer::run_against_mock(executable)?.execve_paths)
}

#[cfg(test)]
mod test_emulate_executable {
    extern crate map_in_place;

    use super::*;
    use crate::utils::testing::TempFile;
    use map_in_place::MapVecInPlace;

    #[test]
    fn returns_the_path_of_the_first_executable_spawned_by_the_script() -> R<()> {
        let script = TempFile::write_temp_script(
            r##"
                #!/usr/bin/env bash

                cd /bin
                ./true
            "##,
        )?;
        assert_eq!(
            emulate_executable(&script.path())?.first().unwrap().command,
            "./true"
        );
        Ok(())
    }

    #[test]
    fn returns_multiple_executables_spawned_by_the_script() -> R<()> {
        let script = TempFile::write_temp_script(
            r##"
                #!/usr/bin/env bash

                cd /bin
                ./true
                ./false
            "##,
        )?;
        assert_eq!(
            emulate_executable(&script.path())?.map(|x| x.command),
            vec!["./true", "./false"]
        );
        Ok(())
    }

    #[test]
    fn works_for_longer_file_names() -> R<()> {
        let long_command = TempFile::new()?;
        copy("/bin/true", long_command.path())?;
        let script = TempFile::write_temp_script(&format!(
            r##"
                #!/usr/bin/env bash

                {}
            "##,
            long_command.path().to_str().unwrap()
        ))?;
        assert_eq!(
            emulate_executable(&script.path())?.map(|x| x.command),
            vec![long_command.path().to_string_lossy()]
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
        let script = TempFile::write_temp_script(&format!(
            r##"
                #!/usr/bin/env bash

                touch {}
            "##,
            testfile.path().to_string_lossy()
        ))?;
        emulate_executable(&script.path())?;
        assert!(!testfile.path().exists(), "touch was executed");
        Ok(())
    }
}
