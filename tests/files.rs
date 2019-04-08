#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use quale::which;
use scriptkeeper::R;
use std::fs;
use tempdir::TempDir;
use test_utils::with_env;
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
            |tests:
            |  - steps:
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
            |tests:
            |  - steps:
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
            |tests:
            |  - steps: []
        ",
        Ok(()),
    )?;
    Ok(())
}

mod executables_that_do_not_exist {
    use super::*;

    #[test]
    fn allows_tests_with_commands_that_do_not_exist() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |does_not_exist
            ",
            r"
                |tests:
                |  - steps:
                |      - does_not_exist
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn does_not_shadow_executables_later_in_the_path() -> R<()> {
        let temp_dir = TempDir::new("test")?;
        fs::copy(
            which("true").ok_or("cannot find true")?,
            temp_dir.path().join("foo"),
        )?;
        with_env(
            "PATH",
            &format!("/bin:{}", temp_dir.path().to_string_lossy()),
            || {
                test_run(
                    r"
                        |#!/usr/bin/env bash
                        |foo
                    ",
                    &format!(
                        r"
                            |tests:
                            |  - env:
                            |      PATH: /bin:{}
                            |    steps:
                            |      - {}/foo
                        ",
                        temp_dir.path().to_string_lossy(),
                        temp_dir.path().to_string_lossy()
                    ),
                    Ok(()),
                )?;
                Ok(())
            },
        )
    }
}
