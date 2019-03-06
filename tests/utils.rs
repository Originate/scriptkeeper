#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![deny(clippy::all)]
#![allow(dead_code)]

use check_protocols::utils::path_to_string;
use check_protocols::{run_check_protocols, Context, ExitCode, R};
use pretty_assertions::assert_eq;
use std::fs;
use tempdir::TempDir;
use test_utils::{run, trim_margin, TempFile};

pub fn test_run_with_tempfile(script: &TempFile, protocol: &str) -> R<(ExitCode, String)> {
    fs::write(
        script.path().with_extension("protocols.yaml"),
        trim_margin(protocol)?,
    )?;
    run_check_protocols(Context::new_test_context(), &script.path())
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

pub fn test_run_c(code: &str, protocol: &str, expected: Result<(), &str>) -> R<()> {
    let tempdir = TempDir::new("test")?;
    let code_file = tempdir.path().join("main.c");
    let executable = tempdir.path().join("main");
    fs::write(&code_file, trim_margin(code)?.as_bytes())?;
    run(
        "gcc",
        vec![
            path_to_string(&code_file)?,
            "-o",
            path_to_string(&executable)?,
            "-Werror",
        ],
    )?;
    fs::write(
        &tempdir.path().join("main.protocols.yaml"),
        trim_margin(&protocol)?,
    )?;
    let result = run_check_protocols(Context::new_test_context(), &executable)?;
    compare_results(result, expected);
    Ok(())
}
