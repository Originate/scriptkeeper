#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use check_protocols::{context::Context, R};
use test_utils::{trim_margin, TempFile};
use utils::{test_run_with_context, test_run_with_tempfile};

#[test]
fn relays_stdout_from_the_tested_script_to_the_user() -> R<()> {
    let context = Context::new_mock();
    let script = TempFile::write_temp_script(
        trim_margin(
            r##"
                |#!/usr/bin/env bash
                |echo foo
            "##,
        )?
        .as_bytes(),
    )?;
    test_run_with_tempfile(
        &context,
        &script,
        r##"
            |protocols:
            |  - protocol: []
        "##,
    )?;
    assert_eq!(context.get_captured_stdout(), "foo\nAll tests passed.\n");
    Ok(())
}

#[test]
fn relays_stderr_from_the_tested_script_to_the_user() -> R<()> {
    let context = Context::new_mock();
    test_run_with_context(
        &context,
        r##"
            |#!/usr/bin/env bash
            |echo foo 1>&2
        "##,
        r##"
            |protocols:
            |  - protocol: []
        "##,
        Ok(()),
    )?;
    assert_eq!(context.get_captured_stderr(), "foo\n");
    Ok(())
}
