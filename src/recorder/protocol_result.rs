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
use std::path::Path;

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
        unmocked_commands: &[Vec<u8>],
    ) -> R<Vec<ProtocolResult>> {
        let mut results = vec![];
        for protocol in protocols.into_iter() {
            results.push(run_against_protocol(
                context,
                &interpreter,
                program,
                &unmocked_commands,
                protocol,
            )?);
        }
        Ok(results)
    }

    pub fn handle_results(
        context: &Context,
        protocols_file: &Path,
        results: &[ProtocolResult],
    ) -> R<ExitCode> {
        ProtocolResult::handle_recorded(context, protocols_file, &results)?;
        ProtocolResult::handle_test_results(context, results)
    }

    fn handle_recorded(
        context: &Context,
        protocols_file: &Path,
        results: &[ProtocolResult],
    ) -> R<()> {
        if results.iter().any(|result| result.is_recorded()) {
            let file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(protocols_file)?;
            write_yaml(
                Box::new(file),
                &Protocols::new(results.iter().map(|result| result.get_protocol()).collect())
                    .serialize(),
            )?;
            writeln!(
                context.stdout(),
                "Protocol holes filled in {}.",
                protocols_file.to_string_lossy()
            )?;
        }
        Ok(())
    }

    fn handle_test_results(context: &Context, results: &[ProtocolResult]) -> R<ExitCode> {
        let test_results = CheckerResults(
            results
                .iter()
                .filter_map(|result| result.get_test_result())
                .collect(),
        );
        write!(context.stdout(), "{}", test_results.format())?;
        Ok(test_results.exitcode())
    }
}

fn run_against_protocol(
    context: &Context,
    interpreter: &Option<Vec<u8>>,
    program: &Path,
    unmocked_commands: &[Vec<u8>],
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
