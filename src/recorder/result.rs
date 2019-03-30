use super::hole_recorder::HoleRecorder;
use crate::context::Context;
use crate::test_checker::{
    checker_result::{CheckerResult, CheckerResults},
    TestChecker,
};
use crate::test_spec::{yaml::write_yaml, Test, Tests};
use crate::tracer::stdio_redirecting::CaptureStderr;
use crate::tracer::Tracer;
use crate::{ExitCode, R};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum RecorderResult {
    Checked(Test, CheckerResult),
    Recorded(Test),
}

impl RecorderResult {
    fn is_recorded(&self) -> bool {
        match self {
            RecorderResult::Recorded(_) => true,
            RecorderResult::Checked(_, _) => false,
        }
    }

    fn get_test(&self) -> Test {
        match self {
            RecorderResult::Checked(test, _) => test.clone(),
            RecorderResult::Recorded(test) => test.clone(),
        }
    }

    fn get_test_result(&self) -> Option<CheckerResult> {
        match self {
            RecorderResult::Checked(_, test_result) => Some(test_result.clone()),
            RecorderResult::Recorded(_) => None,
        }
    }

    pub fn collect_results(
        context: &Context,
        interpreter: &Option<PathBuf>,
        program: &Path,
        tests: Vec<Test>,
        unmocked_commands: &[PathBuf],
    ) -> R<Vec<RecorderResult>> {
        let mut results = vec![];
        for test in tests.into_iter() {
            results.push(run_against_test(
                context,
                &interpreter,
                program,
                unmocked_commands,
                test,
            )?);
        }
        Ok(results)
    }

    pub fn handle_results(
        context: &Context,
        protocols_file: &Path,
        unmocked_commands: Vec<PathBuf>,
        results: &[RecorderResult],
    ) -> R<ExitCode> {
        let checker_results = CheckerResults(
            results
                .iter()
                .filter_map(|result| result.get_test_result())
                .collect(),
        );
        RecorderResult::handle_recorded(
            context,
            protocols_file,
            unmocked_commands,
            &results,
            &checker_results,
        )?;
        write!(context.stdout(), "{}", checker_results.format())?;
        Ok(checker_results.exitcode())
    }

    fn handle_recorded(
        context: &Context,
        protocols_file: &Path,
        unmocked_commands: Vec<PathBuf>,
        results: &[RecorderResult],
        checker_results: &CheckerResults,
    ) -> R<()> {
        if checker_results.is_pass() && results.iter().any(|result| result.is_recorded()) {
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(protocols_file)?;
            write_yaml(
                &mut file,
                &Tests {
                    tests: results.iter().map(|result| result.get_test()).collect(),
                    unmocked_commands,
                    interpreter: None,
                }
                .serialize()?,
            )?;
            writeln!(
                context.stdout(),
                "Test holes filled in {}.",
                protocols_file.to_string_lossy()
            )?;
        }
        Ok(())
    }
}

fn run_against_test(
    context: &Context,
    interpreter: &Option<PathBuf>,
    program: &Path,
    unmocked_commands: &[PathBuf],
    test: Test,
) -> R<RecorderResult> {
    macro_rules! run_against_mock {
        ($syscall_mock:expr) => {
            Tracer::run_against_mock(
                context,
                interpreter,
                program,
                test.arguments.clone(),
                test.env.clone(),
                if test.stderr.is_some() {
                    CaptureStderr::Capture
                } else {
                    CaptureStderr::NoCapture
                },
                $syscall_mock,
            )
        };
    }
    if test.ends_with_hole {
        run_against_mock!(HoleRecorder::new(context, unmocked_commands, test))
    } else {
        Ok(RecorderResult::Checked(
            test.clone(),
            run_against_mock!(TestChecker::new(context, test, unmocked_commands))?,
        ))
    }
}
