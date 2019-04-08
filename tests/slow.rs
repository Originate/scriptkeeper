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
fn allows_tests_with_commands_that_do_not_exist() -> R<()> {
    if option_env!("CI").is_some() {
        for _ in 1..100 {
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
        }
    }
    Ok(())
}
