#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![deny(clippy::all)]

use crate::utils::{test_run, Expect};
use scriptkeeper::R;

#[test]
fn looks_up_step_executable_in_path() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |cp
        ",
        r"
            |steps:
            |  - cp
        ",
        Expect::tests_pass(),
    )?;
    Ok(())
}

#[test]
fn looks_up_unmocked_command_executable_in_path() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |ls > /dev/null
        ",
        r"
            |tests:
            |  - steps: []
            |unmockedCommands:
            |  - ls
        ",
        Expect::tests_pass(),
    )?;
    Ok(())
}

#[test]
fn shortens_received_executable_to_file_name_when_reporting_step_error() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |mv
        ",
        r"
            |steps:
            |  - cp
        ",
        Expect::error_message(
            "
                |error:
                |  expected: cp
                |  received: mv
            ",
        )?,
    )?;
    Ok(())
}

#[test]
fn runs_step_executable_that_is_not_in_path() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |/not/in/path
        ",
        r"
            |steps:
            |  - /not/in/path
        ",
        Expect::tests_pass(),
    )?;
    Ok(())
}
