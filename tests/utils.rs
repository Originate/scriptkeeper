#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![deny(clippy::all)]

use check_protocols::{run_check_protocols, Context, ExitCode, R};
use pretty_assertions::assert_eq;
use std::fs;
use std::io::Cursor;
use std::io::Write;
use test_utils::{trim_margin, TempFile};

pub fn test_run_with_tempfile(script: &TempFile, protocol: &str) -> R<(ExitCode, String)> {
    fs::write(
        script.path().with_extension("protocols.yaml"),
        trim_margin(protocol)?,
    )?;
    with_cursor(|cursor| -> R<ExitCode> {
        run_check_protocols(Context::new_test_context(), &script.path(), cursor)
    })
}

fn compare_results(result: (ExitCode, String), expected: Result<(), &str>) {
    let expected_output = match expected {
        Err(expected_output) => (ExitCode(1), expected_output.to_string()),
        Ok(()) => (ExitCode(0), "All tests passed.\n".to_string()),
    };
    assert_eq!(result, expected_output);
}

pub fn test_run(script_code: &str, protocol: &str, expected: Result<(), &str>) -> R<()> {
    let script = TempFile::write_temp_script(trim_margin(script_code)?.as_bytes())?;
    let result = test_run_with_tempfile(&script, protocol)?;
    compare_results(result, expected);
    Ok(())
}

pub fn with_cursor<A, F>(action: F) -> R<(A, String)>
where
    F: FnOnce(&mut Cursor<Vec<u8>>) -> R<A>,
{
    let mut cursor = Cursor::new(vec![]);
    let result = action(&mut cursor)?;
    Ok((result, String::from_utf8(cursor.into_inner())?))
}
