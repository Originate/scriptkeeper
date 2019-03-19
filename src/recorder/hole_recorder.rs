use super::protocol_result::ProtocolResult;
use crate::context::Context;
use crate::protocol::{Protocol, Protocols};
use crate::protocol_checker::checker_result::CheckerResult;
use crate::protocol_checker::ProtocolChecker;
use crate::recorder::Recorder;
use crate::tracer::stdio_redirecting::Redirector;
use crate::tracer::SyscallMock;
use crate::{ExitCode, R};
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::path::Path;

pub enum HoleRecorder {
    Checker {
        checker: ProtocolChecker,
        original_protocol: Protocol,
    },
    Recorder {
        recorder: Recorder,
    },
}

impl HoleRecorder {
    pub fn new(
        context: &Context,
        unmocked_commands: &[Vec<u8>],
        protocol: Protocol,
    ) -> HoleRecorder {
        HoleRecorder::Checker {
            checker: ProtocolChecker::new(context, protocol.clone(), unmocked_commands),
            original_protocol: protocol,
        }
    }
}

impl SyscallMock for HoleRecorder {
    type Result = ProtocolResult;

    fn handle_execve_enter(
        &mut self,
        pid: Pid,
        registers: &user_regs_struct,
        executable: Vec<u8>,
        arguments: Vec<Vec<u8>>,
    ) -> R<()> {
        match self {
            HoleRecorder::Checker {
                checker,
                original_protocol,
            } => {
                if !checker.protocol.steps.is_empty() {
                    checker.handle_execve_enter(pid, registers, executable, arguments)
                } else {
                    match checker.result {
                        CheckerResult::Failure(_) => {
                            checker.handle_execve_enter(pid, registers, executable, arguments)
                        }
                        CheckerResult::Pass => {
                            *self = HoleRecorder::Recorder {
                                recorder: Recorder::new_with_protocol(original_protocol.clone()),
                            };
                            self.handle_execve_enter(pid, registers, executable, arguments)
                        }
                    }
                }
            }
            HoleRecorder::Recorder { recorder } => {
                recorder.handle_execve_enter(pid, registers, executable, arguments)
            }
        }
    }

    fn handle_exited(&mut self, pid: Pid, exitcode: i32) -> R<()> {
        match self {
            HoleRecorder::Checker { .. } => Ok(()),
            HoleRecorder::Recorder { recorder } => recorder.handle_exited(pid, exitcode),
        }
    }

    fn handle_end(self, exitcode: i32, redirector: &Redirector) -> R<ProtocolResult> {
        Ok(match self {
            HoleRecorder::Checker {
                checker,
                mut original_protocol,
            } => match checker.result {
                CheckerResult::Pass => {
                    original_protocol.ends_with_hole = false;
                    let recorder = Recorder::new_with_protocol(original_protocol);
                    ProtocolResult::Recorded(recorder.handle_end(exitcode, redirector)?)
                }
                failure @ CheckerResult::Failure(_) => {
                    ProtocolResult::Checked(original_protocol, failure)
                }
            },
            HoleRecorder::Recorder { recorder } => {
                ProtocolResult::Recorded(recorder.handle_end(exitcode, redirector)?)
            }
        })
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
