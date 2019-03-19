use super::hole_recorder::HoleRecorder;
use crate::context::Context;
use crate::protocol::{yaml::write_yaml, Protocol, Protocols};
use crate::protocol_checker::{
    checker_result::{CheckerResult, CheckerResults},
    ProtocolChecker,
};
use crate::tracer::stdio_redirecting::CaptureStderr;
use crate::tracer::Tracer;
use crate::{ExitCode, R};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum ProtocolResult {
    Checked(Protocol, CheckerResult),
    Recorded(Protocol),
}

impl ProtocolResult {
    fn is_recorded(&self) -> bool {
        match self {
            ProtocolResult::Recorded(_) => true,
            ProtocolResult::Checked(_, _) => false,
        }
    }

    fn get_protocol(&self) -> Protocol {
        match self {
            ProtocolResult::Checked(protocol, _) => protocol.clone(),
            ProtocolResult::Recorded(protocol) => protocol.clone(),
        }
    }

    fn get_test_result(&self) -> Option<CheckerResult> {
        match self {
            ProtocolResult::Checked(_, test_result) => Some(test_result.clone()),
            ProtocolResult::Recorded(_) => None,
        }
    }

    pub fn collect_results(
        context: &Context,
        interpreter: &Option<Vec<u8>>,
        program: &Path,
        protocols: Vec<Protocol>,
        unmocked_commands: &[PathBuf],
    ) -> R<Vec<ProtocolResult>> {
        let mut results = vec![];
        for protocol in protocols.into_iter() {
            results.push(run_against_protocol(
                context,
                &interpreter,
                program,
                unmocked_commands,
                protocol,
            )?);
        }
        Ok(results)
    }

    pub fn handle_results(
        context: &Context,
        protocols_file: &Path,
        unmocked_commands: Vec<PathBuf>,
        results: &[ProtocolResult],
    ) -> R<ExitCode> {
        let checker_results = CheckerResults(
            results
                .iter()
                .filter_map(|result| result.get_test_result())
                .collect(),
        );
        ProtocolResult::handle_recorded(
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
        results: &[ProtocolResult],
        checker_results: &CheckerResults,
    ) -> R<()> {
        if checker_results.is_pass() && results.iter().any(|result| result.is_recorded()) {
            let file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(protocols_file)?;
            write_yaml(
                Box::new(file),
                &Protocols {
                    protocols: results.iter().map(|result| result.get_protocol()).collect(),
                    unmocked_commands,
                    interpreter: None,
                }
                .serialize()?,
            )?;
            writeln!(
                context.stdout(),
                "Protocol holes filled in {}.",
                protocols_file.to_string_lossy()
            )?;
        }
        Ok(())
    }
}

fn run_against_protocol(
    context: &Context,
    interpreter: &Option<Vec<u8>>,
    program: &Path,
    unmocked_commands: &[PathBuf],
    protocol: Protocol,
) -> R<ProtocolResult> {
    macro_rules! run_against_mock {
        ($syscall_mock:expr) => {
            Tracer::run_against_mock(
                context,
                interpreter,
                program,
                protocol.arguments.clone(),
                protocol.env.clone(),
                if protocol.stderr.is_some() {
                    CaptureStderr::Capture
                } else {
                    CaptureStderr::NoCapture
                },
                $syscall_mock,
            )
        };
    }
    if protocol.ends_with_hole {
        run_against_mock!(HoleRecorder::new(context, unmocked_commands, protocol))
    } else {
        Ok(ProtocolResult::Checked(
            protocol.clone(),
            run_against_mock!(ProtocolChecker::new(context, protocol, unmocked_commands))?,
        ))
    }
}
