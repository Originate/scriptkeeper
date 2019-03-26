pub mod hole_recorder;
mod protocol_result;

use crate::protocol::command::Command;
use crate::protocol::command_matcher::CommandMatcher;
use crate::protocol::{compare_executables, Protocol, Step};
use crate::tracer::stdio_redirecting::Redirector;
use crate::tracer::SyscallMock;
use crate::R;
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::ffi::OsString;
use std::path::PathBuf;

pub struct Recorder {
    protocol: Protocol,
    command: Option<Command>,
    unmocked_commands: Vec<PathBuf>,
    mocked_executables: Vec<PathBuf>,
}

impl Recorder {
    pub fn empty() -> Recorder {
        Recorder {
            protocol: Protocol::new(vec![]),
            command: None,
            unmocked_commands: vec![],
            mocked_executables: vec![],
        }
    }

    pub fn new(
        protocol: Protocol,
        unmocked_commands: &[PathBuf],
        mocked_executables: &[PathBuf],
    ) -> Recorder {
        Recorder {
            protocol,
            command: None,
            unmocked_commands: unmocked_commands.to_vec(),
            mocked_executables: mocked_executables.to_vec(),
        }
    }
}

impl SyscallMock for Recorder {
    type Result = Protocol;

    fn handle_execve_enter(
        &mut self,
        _pid: Pid,
        _registers: &user_regs_struct,
        executable: PathBuf,
        arguments: Vec<OsString>,
    ) -> R<()> {
        let is_unmocked_command = self.unmocked_commands.iter().any(|unmocked_command| {
            compare_executables(&self.mocked_executables, unmocked_command, &executable)
        });
        if !is_unmocked_command {
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
            self.protocol.steps.push_back(Step {
                command_matcher: CommandMatcher::ExactMatch(command),
                stdout: vec![],
                exitcode,
            });
        }
        Ok(())
    }

    fn handle_end(mut self, exitcode: i32, _redirector: &Redirector) -> R<Protocol> {
        if exitcode != 0 {
            self.protocol.exitcode = Some(exitcode);
        }
        Ok(self.protocol)
    }
}
