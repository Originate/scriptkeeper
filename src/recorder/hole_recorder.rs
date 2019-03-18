use super::protocol_result::ProtocolResult;
use crate::context::Context;
use crate::protocol::{Protocol, Protocols};
use crate::recorder::Recorder;
use crate::tracer::stdio_redirecting::Redirector;
use crate::tracer::SyscallMock;
use crate::{ExitCode, R};
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::path::Path;

pub struct HoleRecorder {
    recorder: Recorder,
}

impl HoleRecorder {
    pub fn new() -> HoleRecorder {
        HoleRecorder {
            recorder: Recorder::default(),
        }
    }
}

impl SyscallMock for HoleRecorder {
    type Result = Protocol;

    fn handle_execve_enter(
        &mut self,
        pid: Pid,
        registers: &user_regs_struct,
        executable: Vec<u8>,
        arguments: Vec<Vec<u8>>,
    ) -> R<()> {
        self.recorder
            .handle_execve_enter(pid, registers, executable, arguments)
    }

    fn handle_exited(&mut self, pid: Pid, exitcode: i32) -> R<()> {
        self.recorder.handle_exited(pid, exitcode)
    }

    fn handle_end(self, exitcode: i32, redirector: &Redirector) -> R<Protocol> {
        self.recorder.handle_end(exitcode, redirector)
    }
}

pub fn run_against_protocols(
    context: &Context,
    program: &Path,
    protocols_file: &Path,
    Protocols {
        protocols,
        unmocked_commands,
        interpreter,
    }: Protocols,
) -> R<ExitCode> {
    let results = ProtocolResult::collect_results(
        context,
        &interpreter,
        program,
        protocols,
        &unmocked_commands,
    )?;
    ProtocolResult::handle_results(context, protocols_file, &results)
}
