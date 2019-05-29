#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use scriptkeeper::context::Context;
use scriptkeeper::{cli, run_main, R};
use test_utils::{trim_margin, TempFile};
use utils::assert_eq_yaml;

mod yaml_formatting {
    use super::*;

    #[test]
    fn output_contains_trailing_newline() -> R<()> {
        let context = Context::new_mock();
        run_main(
            &context,
            &cli::Args::Scriptkeeper {
                script_path: TempFile::write_temp_script(b"#!/usr/bin/env bash")?.path(),
                record: true,
            },
        )?;
        assert!(context.get_captured_stdout().ends_with('\n'));
        Ok(())
    }

    #[test]
    fn does_not_output_three_leading_dashes() -> R<()> {
        let context = Context::new_mock();
        run_main(
            &context,
            &cli::Args::Scriptkeeper {
                script_path: TempFile::write_temp_script(b"#!/usr/bin/env bash")?.path(),
                record: true,
            },
        )?;
        assert!(!context.get_captured_stdout().starts_with("---"));
        Ok(())
    }
}

fn test_recording(script: &str, expected: &str) -> R<()> {
    let script = TempFile::write_temp_script(trim_margin(script)?.as_bytes())?;
    let context = Context::new_mock();
    run_main(
        &context,
        &cli::Args::Scriptkeeper {
            script_path: script.path(),
            record: true,
        },
    )?;
    let output = context.get_captured_stdout();
    assert_eq_yaml(&output, &trim_margin(expected)?)?;
    Ok(())
}

#[test]
fn records_an_empty_test() -> R<()> {
    test_recording(
        "
            |#!/usr/bin/env bash
        ",
        "
            |tests:
            |  - steps: []
        ",
    )
}

#[test]
fn records_test_steps() -> R<()> {
    test_recording(
        "
            |#!/usr/bin/env bash
            |ls >/dev/null
        ",
        "
            |tests:
            |  - steps:
            |      - ls
        ",
    )
}

#[test]
fn records_multiple_steps() -> R<()> {
    test_recording(
        "
            |#!/usr/bin/env bash
            |date > /dev/null
            |ls > /dev/null
        ",
        "
            |tests:
            |  - steps:
            |      - date
            |      - ls
        ",
    )
}

#[test]
fn records_command_arguments() -> R<()> {
    test_recording(
        "
            |#!/usr/bin/env bash
            |date --version > /dev/null
        ",
        r#"
            |tests:
            |  - steps:
            |      - "date --version"
        "#,
    )
}

#[test]
fn records_script_exitcode() -> R<()> {
    test_recording(
        "
            |#!/usr/bin/env bash
            |exit 42
        ",
        "
            |tests:
            |  - steps: []
            |    exitcode: 42
        ",
    )
}

#[test]
fn records_command_exitcodes() -> R<()> {
    test_recording(
        r#"
            |#!/usr/bin/env bash
            |bash -c "exit 42"
            |true
        "#,
        r#"
            |tests:
            |  - steps:
            |      - command: bash -c "exit 42"
            |        exitcode: 42
        "#,
    )
}
