#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

use check_protocols::cli;
use check_protocols::context::Context;
use check_protocols::{run_main, R};
use pretty_assertions::assert_eq;
use test_utils::{trim_margin, TempFile};
use yaml_rust::YamlLoader;

fn assert_eq_yaml(result: &str, expected: &str) -> R<()> {
    let result =
        YamlLoader::load_from_str(result).map_err(|error| format!("{}\n({})", error, result))?;
    let expected = trim_margin(expected)?;
    let expected = YamlLoader::load_from_str(&expected)
        .map_err(|error| format!("{}\n({})", error, expected))?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn records_an_empty_protocol() -> R<()> {
    let script = TempFile::write_temp_script(
        trim_margin(
            "
                |#!/usr/bin/env bash
            ",
        )?
        .as_bytes(),
    )?;
    let context = Context::new_mock();
    run_main(
        &context,
        &cli::Args::CheckProtocols {
            script_path: script.path(),
            record: true,
        },
    )?;
    let output = context.get_captured_stdout();
    assert_eq_yaml(
        &output,
        "
            |protocols:
            |  - protocol: []
        ",
    )?;
    Ok(())
}

#[test]
fn records_protocol_steps() -> R<()> {
    let script = TempFile::write_temp_script(
        trim_margin(
            "
                |#!/usr/bin/env bash
                |/bin/true
            ",
        )?
        .as_bytes(),
    )?;
    let context = Context::new_mock();
    run_main(
        &context,
        &cli::Args::CheckProtocols {
            script_path: script.path(),
            record: true,
        },
    )?;
    let output = context.get_captured_stdout();
    assert_eq_yaml(
        &output,
        "
            |protocols:
            |  - protocol:
            |      - /bin/true
        ",
    )?;
    Ok(())
}
