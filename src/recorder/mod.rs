pub mod hole_recorder;
mod protocol_result;

use crate::protocol::command::Command;
use crate::protocol::{Protocol, Step};
use crate::tracer::stdio_redirecting::Redirector;
use crate::tracer::SyscallMock;
use crate::R;
use libc::user_regs_struct;
use nix::unistd::Pid;

pub struct Recorder {
    protocol: Protocol,
    command: Option<Command>,
}

impl Recorder {
    pub fn new() -> Recorder {
        Recorder {
            protocol: Protocol::new(vec![]),
            command: None,
        }
    }

    pub fn new_with_protocol(protocol: Protocol) -> Recorder {
        Recorder {
            protocol,
            command: None,
        }
    }
}

impl SyscallMock for Recorder {
    type Result = Protocol;

    fn handle_execve_enter(
        &mut self,
        _pid: Pid,
        _registers: &user_regs_struct,
        executable: Vec<u8>,
        arguments: Vec<Vec<u8>>,
    ) -> R<()> {
        self.command = Some(Command {
            executable,
            arguments,
        });
        Ok(())
    }

    fn handle_exited(&mut self, _pid: Pid, exitcode: i32) -> R<()> {
        let command = self
            .command
            .clone()
            .ok_or("Recorder.handle_execve_exit: command not set")?;
        self.command = None;
        self.protocol.steps.push_back(Step {
            command,
            stdout: vec![],
            exitcode,
        });
        Ok(())
    }

    fn handle_end(mut self, exitcode: i32, _redirector: &Redirector) -> R<Protocol> {
        if exitcode != 0 {
            self.protocol.exitcode = Some(exitcode);
        }
        Ok(self.protocol)
    }
}
