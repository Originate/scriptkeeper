#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use scriptkeeper::R;
use utils::test_run;

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
            |protocols:
            |  - protocol:
            |      - cp
            |    mockedFiles:
            |      - /foo
        ",
        Ok(()),
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
            |protocols:
            |  - protocol:
            |      - command: cp
            |    mockedFiles:
            |      - /foo/
        ",
        Ok(()),
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
            |protocols:
            |  - protocol: []
        ",
        Ok(()),
    )?;
    Ok(())
}

#[test]
fn allows_to_mock_executable_existence() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |/bin/does_not_exist
        ",
        r"
            |protocols:
            |  - protocol:
            |      - /bin/does_not_exist
            |mockedExecutables:
            |  - does_not_exist
        ",
        Ok(()),
    )?;
    Ok(())
}

#[test]
fn works_with_absolute_executables() {}
