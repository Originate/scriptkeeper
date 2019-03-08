#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]
#![allow(dead_code)]

use check_protocols::{context::Context, run_check_protocols, ExitCode, R};
use pretty_assertions::assert_eq;
use std::fs;
use std::io::Cursor;
use test_utils::{trim_margin, TempFile};

fn compare_results(result: (ExitCode, String), expected: Result<(), &str>) {
    let expected_output = match expected {
        Err(expected_output) => (ExitCode(1), expected_output.to_string()),
        Ok(()) => (ExitCode(0), "All tests passed.\n".to_string()),
    };
    assert_eq!(result, expected_output);
}

pub fn test_run_with_tempfile(
    context: &Context,
    script: &TempFile,
    protocol: &str,
) -> R<(ExitCode, String)> {
    fs::write(
        script.path().with_extension("protocols.yaml"),
        trim_margin(protocol)?,
    )?;
    with_cursor(|cursor| -> R<ExitCode> { run_check_protocols(context, &script.path(), cursor) })
}

pub fn test_run_with_context(
    context: &Context,
    script_code: &str,
    protocol: &str,
    expected: Result<(), &str>,
) -> R<()> {
    let script = TempFile::write_temp_script(trim_margin(script_code)?.as_bytes())?;
    let result = test_run_with_tempfile(context, &script, protocol)?;
    compare_results(result, expected);
    Ok(())
}

pub fn test_run(script_code: &str, protocol: &str, expected: Result<(), &str>) -> R<()> {
    test_run_with_context(&Context::new_mock(), script_code, protocol, expected)
}

pub fn with_cursor<A, F>(action: F) -> R<(A, String)>
where
    F: FnOnce(&mut Cursor<Vec<u8>>) -> R<A>,
{
    let mut cursor = Cursor::new(vec![]);
    let result = action(&mut cursor)?;
    Ok((result, String::from_utf8(cursor.into_inner())?))
}
