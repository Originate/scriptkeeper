#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

use crate::utils::{test_run, Expect};
use scriptkeeper::R;

#[test]
fn allows_to_mock_files_existence() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |if [ -f /foo ]; then
            |  cp
            |fi
        ",
        r"
            |tests:
            |  - steps:
            |      - cp
            |    mockedFiles:
            |      - /foo
        ",
        Expect::tests_pass(),
    )?;
    Ok(())
}

#[test]
fn allows_to_mock_directory_existence() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |if [ -d /foo/ ]; then
            |  cp
            |fi
        ",
        r"
            |tests:
            |  - steps:
            |      - command: cp
            |    mockedFiles:
            |      - /foo/
        ",
        Expect::tests_pass(),
    )?;
    Ok(())
}

#[test]
fn does_not_mock_existence_of_unspecified_files() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |if [ -f /foo ]; then
            |  cp
            |fi
        ",
        r"
            |tests:
            |  - steps: []
        ",
        Expect::tests_pass(),
    )?;
    Ok(())
}
