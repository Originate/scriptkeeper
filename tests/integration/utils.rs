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

#[derive(Debug, PartialEq)]
pub struct Expect {
    expected_exit_code: ExitCode,
    expected_stdout: String,
    expected_stderr: String,
}

impl Expect {
    pub fn tests_pass() -> Self {
        Expect {
            expected_exit_code: ExitCode(0),
            expected_stdout: "All tests passed.\n".to_string(),
            expected_stderr: "".to_string(),
        }
    }

    pub fn error_message(expected_output: &str) -> R<Self> {
        Ok(Expect {
            expected_exit_code: ExitCode(1),
            expected_stdout: trim_margin(expected_output)?,
            expected_stderr: "".to_string(),
        })
    }

    pub fn with_stdout(self, expected_output: &str) -> Self {
        Expect {
            expected_stdout: expected_output.to_string(),
            ..self
        }
    }

    pub fn with_stderr(self, expected_output: &str) -> Self {
        Expect {
            expected_stderr: expected_output.to_string(),
            ..self
        }
    }
}

pub fn prepare_script(script_code: &str, tests: &str) -> R<(TempFile, PathBuf)> {
    let script = TempFile::write_temp_script(trim_margin(script_code)?.as_bytes())?;
    let test_file = format!("{}.test.yaml", path_to_string(&script.path())?);
    fs::write(&test_file, trim_margin(tests)?)?;
    Ok((script, PathBuf::from(test_file)))
}

pub fn test_run_with_tempfile(context: &Context, script: &TempFile, tests: &str) -> R<ExitCode> {
    fs::write(
        script.path().with_extension("test.yaml"),
        trim_margin(tests)?,
    )?;
    run_scriptkeeper(context, &script.path())
}

pub fn test_run(script_code: &str, tests: &str, expected: Expect) -> R<()> {
    let context = &Context::new_mock();
    let script = TempFile::write_temp_script(trim_margin(script_code)?.as_bytes())?;
    let exit_code = test_run_with_tempfile(context, &script, tests)?;
    assert_eq!(
        Expect {
            expected_exit_code: exit_code,
            expected_stdout: context.get_captured_stdout(),
            expected_stderr: context.get_captured_stderr(),
        },
        expected,
    );
    Ok(())
}

pub fn assert_eq_yaml(result: &str, expected: &str) -> R<()> {
    let result =
        YamlLoader::load_from_str(result).map_err(|error| format!("{}\n({})", error, result))?;
    let expected = YamlLoader::load_from_str(expected)
        .map_err(|error| format!("{}\n({})", error, expected))?;
    assert_eq!(result, expected);
    Ok(())
}
