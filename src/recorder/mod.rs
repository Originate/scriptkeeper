pub mod hole_recorder;
mod result;

use crate::context::Context;
use crate::executable_mock::ExecutableMock;
use crate::test_spec::command::Command;
use crate::test_spec::command_matcher::CommandMatcher;
use crate::test_spec::{compare_executables, Step, Test};
use crate::tracer::stdio_redirecting::Redirector;
use crate::tracer::SyscallMock;
use crate::R;
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::ffi::OsString;
use std::path::PathBuf;

pub struct Recorder {
    context: Context,
    test: Test,
    command: Option<Command>,
    unmocked_commands: Vec<PathBuf>,
    temporary_executables: Vec<ExecutableMock>,
}

impl Recorder {
    pub fn empty(context: &Context) -> Recorder {
        // fixme: use new
        Recorder {
            context: context.clone(),
            test: Test::new(vec![]),
            command: None,
            unmocked_commands: vec![],
            temporary_executables: vec![],
        }
    }

    pub fn new(context: &Context, test: Test, unmocked_commands: &[PathBuf]) -> Recorder {
        Recorder {
            context: context.clone(),
            test,
            command: None,
            unmocked_commands: unmocked_commands.to_vec(),
            temporary_executables: vec![],
        }
    }
}

impl SyscallMock for Recorder {
    type Result = Test;

    fn handle_execve_enter(
        &mut self,
        pid: Pid,
        registers: &user_regs_struct,
        executable: PathBuf,
        arguments: Vec<OsString>,
    ) -> R<()> {
        let is_unmocked_command = self
            .unmocked_commands
            .iter()
            .any(|unmocked_command| compare_executables(unmocked_command, &executable));
        if !is_unmocked_command {
            let executable_mock_path = ExecutableMock::wrapper(&self.context, &executable)?;
            let path = executable_mock_path.path();
            self.temporary_executables.push(executable_mock_path);
            ExecutableMock::poke_for_execve_syscall(pid, registers, path)?;

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
