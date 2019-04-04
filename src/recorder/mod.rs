pub mod hole_recorder;
mod result;

use crate::test_spec::command::Command;
use crate::test_spec::command_matcher::CommandMatcher;
use crate::test_spec::executable_path::is_unmocked_command;
use crate::test_spec::{Step, Test};
use crate::tracer::stdio_redirecting::Redirector;
use crate::tracer::SyscallMock;
use crate::R;
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::ffi::OsString;
use std::path::PathBuf;

pub struct Recorder {
    test: Test,
    command: Option<Command>,
    unmocked_commands: Vec<PathBuf>,
}

impl Recorder {
    pub fn empty() -> Recorder {
        Recorder {
            test: Test::new(vec![]),
            command: None,
            unmocked_commands: vec![],
        }
    }

    pub fn new(test: Test, unmocked_commands: &[PathBuf]) -> Recorder {
        Recorder {
            test,
            command: None,
            unmocked_commands: unmocked_commands.to_vec(),
        }
    }
}

impl SyscallMock for Recorder {
    type Result = Test;

    fn handle_execve_enter(
        &mut self,
        _pid: Pid,
        _registers: &user_regs_struct,
        executable: PathBuf,
        arguments: Vec<OsString>,
    ) -> R<()> {
        if !is_unmocked_command(&self.unmocked_commands, &executable) {
            self.command = Some(Command {
                executable,
                arguments,
            });
        }
        Ok(())
    }

    fn handle_exited(&mut self, _pid: Pid, exitcode: i32) -> R<()> {
        if let Some(command) = self.command.clone() {
            self.command = None;
            self.test.steps.push_back(Step {
                command_matcher: CommandMatcher::ExactMatch(command),
                stdout: vec![],
                exitcode,
            });
        }
        Ok(())
    }

    fn handle_end(mut self, exitcode: i32, _redirector: &Redirector) -> R<Test> {
        if exitcode != 0 {
            self.test.exitcode = Some(exitcode);
        }
        Ok(self.test)
    }
}
