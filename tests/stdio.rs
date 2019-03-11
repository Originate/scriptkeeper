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
use utils::{test_run, test_run_with_context, test_run_with_tempfile};

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

mod expected_stderr {
    use super::*;

    #[test]
    fn fails_when_not_matching() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |echo bar 1>&2
            "##,
            r##"
                |protocols:
                |  - protocol: []
                |    stderr: "foo\n"
            "##,
            Err(&trim_margin(
                r##"
                    |error:
                    |  expected output to stderr: "foo\n"
                    |  received output to stderr: "bar\n"
                "##,
            )?),
        )?;
        Ok(())
    }

    #[test]
    fn passes_when_matching() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |echo foo 1>&2
            "##,
            r##"
                |protocols:
                |  - protocol: []
                |    stderr: "foo\n"
            "##,
            Ok(()),
        )?;
        Ok(())
    }
}
