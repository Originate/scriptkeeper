#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use scriptkeeper::R;
use test_utils::trim_margin;
use utils::{test_run, Expect};

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
        Expect::ok(),
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
        Expect::ok(),
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
        Expect::err(&trim_margin(
            "
                |error:
                |  expected: cp
                |  received: mv
            ",
        )?),
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
        Expect::ok(),
    )?;
    Ok(())
}
