#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use scriptkeeper::R;
use test_utils::trim_margin;
use utils::{test_run, Expect};

#[test]
fn relays_stdout_from_the_tested_script_to_the_user() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |echo foo
        ",
        r"
            |tests:
            |  - steps: []
        ",
        Expect::ok().stdout("foo\nAll tests passed.\n"),
    )?;
    Ok(())
}

#[test]
fn relays_stderr_from_the_tested_script_to_the_user() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |echo foo 1>&2
        ",
        r"
            |tests:
            |  - steps: []
        ",
        Expect::ok().stderr("foo\n"),
    )?;
    Ok(())
}

mod expected_stdout {
    use super::*;

    #[test]
    fn fails_when_not_matching() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |echo bar
            ",
            r#"
                |tests:
                |  - steps: []
                |    stdout: "foo\n"
            "#,
            Expect::err(&trim_margin(
                r#"
                    |bar
                    |error:
                    |  expected output to stdout: "foo\n"
                    |  received output to stdout: "bar\n"
                "#,
            )?),
        )?;
        Ok(())
    }

    #[test]
    fn passes_when_matching() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |echo foo
            ",
            r#"
                |tests:
                |  - steps: []
                |    stdout: "foo\n"
            "#,
            Expect::ok().stdout("foo\nAll tests passed.\n"),
        )?;
        Ok(())
    }

    #[test]
    fn fails_when_expecting_stdout_but_none_printed() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
            ",
            r#"
                |tests:
                |  - steps: []
                |    stdout: "foo\n"
            "#,
            Expect::err(&trim_margin(
                r#"
                    |error:
                    |  expected output to stdout: "foo\n"
                    |  received output to stdout: ""
                "#,
            )?),
        )?;
        Ok(())
    }

    #[test]
    fn when_not_specified_but_scripts_writes_to_stdout() -> R<()> {
        let result = test_run_with_tempfile(
            &Context::new_mock(),
            &TempFile::write_temp_script(
                trim_margin(
                    r"
                        |#!/usr/bin/env bash
                        |echo foo
                    ",
                )?
                .as_bytes(),
            )?,
            r#"
                |protocols:
                |  - protocol: []
            "#,
        )?;
        assert_eq!(result.0, ExitCode(0));
        assert_eq!(result.1, "foo\nAll tests passed.\n");
        Ok(())
    }
}

mod expected_stderr {
    use super::*;

    #[test]
    fn fails_when_not_matching() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |echo bar 1>&2
            ",
            r#"
                |tests:
                |  - steps: []
                |    stderr: "foo\n"
            "#,
            Expect::err(&trim_margin(
                r#"
                    |error:
                    |  expected output to stderr: "foo\n"
                    |  received output to stderr: "bar\n"
                "#,
            )?)
            .stderr("bar\n"),
        )?;
        Ok(())
    }

    #[test]
    fn passes_when_matching() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |echo foo 1>&2
            ",
            r#"
                |tests:
                |  - steps: []
                |    stderr: "foo\n"
            "#,
            Expect::ok().stderr("foo\n"),
        )?;
        Ok(())
    }

    #[test]
    fn fails_when_expecting_stderr_but_none_printed() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
            ",
            r#"
                |tests:
                |  - steps: []
                |    stderr: "foo\n"
            "#,
            Expect::err(&trim_margin(
                r#"
                    |error:
                    |  expected output to stderr: "foo\n"
                    |  received output to stderr: ""
                "#,
            )?),
        )?;
        Ok(())
    }

    #[test]
    fn when_not_specified_but_scripts_writes_to_stderr() {}
}
