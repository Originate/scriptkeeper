#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]
#![allow(dead_code)]

use pretty_assertions::assert_eq;
use scriptkeeper::utils::path_to_string;
use scriptkeeper::{context::Context, run_scriptkeeper, ExitCode, R};
use std::fs;
use std::path::PathBuf;
use test_utils::{trim_margin, TempFile};
use yaml_rust::YamlLoader;

fn compare_results(result: (ExitCode, String), expected: Result<(), &str>) {
    let expected_output = match expected {
        Err(expected_output) => (ExitCode(1), expected_output.to_string()),
        Ok(()) => (ExitCode(0), "All tests passed.\n".to_string()),
    };
    assert_eq!(result, expected_output);
}

pub fn prepare_script(script_code: &str, protocol: &str) -> R<(TempFile, PathBuf)> {
    let script = TempFile::write_temp_script(trim_margin(script_code)?.as_bytes())?;
    let protocols_file = format!("{}.protocols.yaml", path_to_string(&script.path())?);
    fs::write(&protocols_file, trim_margin(protocol)?)?;
    Ok((script, PathBuf::from(protocols_file)))
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
    let exitcode = run_scriptkeeper(context, &script.path())?;
    Ok((exitcode, context.get_captured_stdout()))
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

pub fn assert_eq_yaml(result: &str, expected: &str) -> R<()> {
    let result =
        YamlLoader::load_from_str(result).map_err(|error| format!("{}\n({})", error, result))?;
    let expected = YamlLoader::load_from_str(expected)
        .map_err(|error| format!("{}\n({})", error, expected))?;
    assert_eq!(result, expected);
    Ok(())
}
