use crate::context::Context;
use crate::recorder::result::RecorderResult;
use crate::recorder::Recorder;
use crate::test_checker::checker_result::CheckerResult;
use crate::test_checker::TestChecker;
use crate::test_spec::{Test, Tests};
use crate::tracer::stdio_redirecting::Redirector;
use crate::tracer::SyscallMock;
use crate::{ExitCode, R};
use libc::user_regs_struct;
use nix::unistd::Pid;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub enum HoleRecorder {
    Checker {
        checker: TestChecker,
        original_test: Test,
    },
    Recorder {
        recorder: Recorder,
    },
}

impl HoleRecorder {
    pub fn new(context: &Context, unmocked_commands: &[PathBuf], test: Test) -> HoleRecorder {
        HoleRecorder::Checker {
            checker: TestChecker::new(context, test.clone(), unmocked_commands),
            original_test: test,
        }
    }
}

impl SyscallMock for HoleRecorder {
    type Result = RecorderResult;

    fn handle_execve_enter(
        &mut self,
        pid: Pid,
        registers: &user_regs_struct,
        executable: PathBuf,
        arguments: Vec<OsString>,
    ) -> R<()> {
        match self {
            HoleRecorder::Checker {
                checker,
                original_test,
            } => {
                if !checker.test.steps.is_empty() {
                    checker.handle_execve_enter(pid, registers, executable, arguments)
                } else {
                    match checker.result {
                        CheckerResult::Failure(_) => {
                            checker.handle_execve_enter(pid, registers, executable, arguments)
                        }
                        CheckerResult::Pass => {
                            *self = HoleRecorder::Recorder {
                                recorder: Recorder::new(
                                    original_test.clone(),
                                    &checker.unmocked_commands,
                                ),
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

    fn handle_end(self, exitcode: i32, redirector: &Redirector) -> R<RecorderResult> {
        Ok(match self {
            HoleRecorder::Checker {
                checker,
                mut original_test,
            } => match checker.result {
                CheckerResult::Pass => {
                    original_test.ends_with_hole = false;
                    let recorder = Recorder::new(original_test, &checker.unmocked_commands);
                    RecorderResult::Recorded(recorder.handle_end(exitcode, redirector)?)
                }
                failure @ CheckerResult::Failure(_) => {
                    RecorderResult::Checked(original_test, failure)
                }
            },
            HoleRecorder::Recorder { recorder } => {
                RecorderResult::Recorded(recorder.handle_end(exitcode, redirector)?)
            }
        })
    }
}

pub fn run_against_tests(
    context: &Context,
    program: &Path,
    test_file: &Path,
    Tests {
        tests,
        unmocked_commands,
        interpreter,
    }: Tests,
) -> R<ExitCode> {
    let results =
        RecorderResult::collect_results(context, &interpreter, program, tests, &unmocked_commands)?;
    RecorderResult::handle_results(context, test_file, unmocked_commands, &results)
}
