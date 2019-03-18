#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use check_protocols::context::Context;
use check_protocols::utils::path_to_string;
use check_protocols::{cli, run_main, R};
use pretty_assertions::assert_eq;
use std::fs;
use test_utils::trim_margin;
use utils::{assert_eq_yaml, prepare_script};

fn test_holes(script_code: &str, existing: &str, expected: &str) -> R<()> {
    let (script, protocols_file) = prepare_script(script_code, existing)?;
    run_main(
        &Context::new_mock(),
        &cli::Args::CheckProtocols {
            script_path: script.path(),
            record: false,
        },
    )?;
    assert_eq_yaml(
        &trim_margin(expected)?,
        &String::from_utf8(fs::read(&protocols_file)?)?,
    )?;
    Ok(())
}

#[test]
fn fills_in_holes_in_protocols_files() -> R<()> {
    test_holes(
        "
            |#!/usr/bin/env bash
            |ls
        ",
        "
            |protocols:
            |  - protocol:
            |      - _
        ",
        "
            |protocols:
            |  - protocol:
            |      - ls
        ",
    )
}

#[test]
fn indicates_on_stdout_that_the_protocols_file_was_written_to() -> R<()> {
    let (script, protocols_file) = prepare_script(
        "
            |#!/usr/bin/env bash
            |/bin/true
        ",
        "
            |protocols:
            |  - protocol:
            |      - _
        ",
    )?;
    let context = Context::new_mock();
    run_main(
        &context,
        &cli::Args::CheckProtocols {
            script_path: script.path(),
            record: false,
        },
    )?;
    assert_eq!(
        context.get_captured_stdout(),
        format!(
            "Protocol holes filled in {}.\nAll tests passed.\n",
            path_to_string(&protocols_file)?
        )
    );
    Ok(())
}

#[test]
fn does_not_modify_files_without_holes() -> R<()> {
    let (script, protocols_file) = prepare_script(
        "
            |#!/usr/bin/env bash
            |/bin/true
        ",
        "
            |protocols:
            |  - protocol:
            |      - /bin/true
        ",
    )?;
    let old_modification_time = fs::metadata(&protocols_file)?.modified()?;
    run_main(
        &Context::new_mock(),
        &cli::Args::CheckProtocols {
            script_path: script.path(),
            record: false,
        },
    )?;
    let new_modification_time = fs::metadata(&protocols_file)?.modified()?;
    assert_eq!(new_modification_time, old_modification_time);
    Ok(())
}

#[test]
fn works_for_holes_following_specified_steps() -> R<()> {
    test_holes(
        "
            |#!/usr/bin/env bash
            |ls
            |ls -la
        ",
        "
            |protocols:
            |  - protocol:
            |      - ls
            |      - _
        ",
        "
            |protocols:
            |  - protocol:
            |      - ls
            |      - ls -la
        ",
    )
}

#[test]
fn works_in_conjunction_with_protocols_without_hole() -> R<()> {
    test_holes(
        "
            |#!/usr/bin/env bash
            |if [ $1 == foo ] ; then
            |  ls
            |else
            |  ls -la
            |fi
        ",
        "
            |protocols:
            |  - arguments: foo
            |    protocol:
            |      - ls
            |  - protocol:
            |      - _
        ",
        "
            |protocols:
            |  - arguments: foo
            |    protocol:
            |      - ls
            |  - protocol:
            |      - ls -la
        ",
    )
}

#[test]
fn works_for_multiple_protocols_with_holes() -> R<()> {
    test_holes(
        "
            |#!/usr/bin/env bash
            |if [ $1 == foo ] ; then
            |  ls
            |else
            |  ls -la
            |fi
        ",
        "
            |protocols:
            |  - arguments: foo
            |    protocol:
            |      - _
            |  - protocol:
            |      - _
        ",
        "
            |protocols:
            |  - arguments: foo
            |    protocol:
            |      - ls
            |  - protocol:
            |      - ls -la
        ",
    )
}

#[test]
fn preserves_script_arguments() -> R<()> {
    test_holes(
        "
            |#!/usr/bin/env bash
            |if [ $1 == foo ] ; then
            |  ls > /dev/null
            |fi
        ",
        "
            |protocols:
            |  - arguments: foo
            |    protocol:
            |      - _
        ",
        "
            |protocols:
            |  - arguments: foo
            |    protocol:
            |      - ls
        ",
    )
}

#[test]
fn errors_in_protocol_with_hole() -> R<()> {
    let (script, _) = prepare_script(
        "
            |#!/usr/bin/env bash
            |ls > /dev/null
            |ls > /dev/null
        ",
        "
            |protocols:
            |  - arguments: foo
            |    protocol:
            |      - ls -la
            |      - _
        ",
    )?;
    let context = Context::new_mock();
    run_main(
        &context,
        &cli::Args::CheckProtocols {
            script_path: script.path(),
            record: false,
        },
    )?;
    assert_eq!(
        context.get_captured_stdout(),
        "error:\n  expected: ls -la\n  received: ls\n"
    );
    Ok(())
}
