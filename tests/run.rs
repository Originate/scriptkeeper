use check_protocols::{run, R};
use std::fs;
use test_utils::{trim_margin, TempFile};

fn test_run(script_code: &str, protocol: &str, expected_output: &str) -> R<()> {
    let script = TempFile::write_temp_script(&trim_margin(script_code)?)?;
    fs::write(
        script.path().with_extension("protocol.yaml"),
        trim_margin(protocol)?,
    )?;
    let output = run(&script.path())?;
    assert_eq!(output, expected_output);
    Ok(())
}

#[test]
fn simple() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |
            |/bin/true
        "##,
        r##"
            |- /bin/true
        "##,
        "All tests passed.\n",
    )?;
    Ok(())
}

#[test]
fn multiple() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |
            |/bin/true
            |/bin/ls > /dev/null
        "##,
        r##"
            |- /bin/true
            |- /bin/ls
        "##,
        "All tests passed.\n",
    )?;
    Ok(())
}

#[test]
fn arguments() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |
            |/bin/true foo
        "##,
        r##"
            |- /bin/true foo
        "##,
        "All tests passed.\n",
    )?;
    Ok(())
}

#[test]
fn failing() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |
            |/bin/false
        "##,
        r##"
            |- /bin/true
        "##,
        &trim_margin(
            "
                |error:
                |expected: /bin/true
                |received: /bin/false
            ",
        )?,
    )?;
    Ok(())
}

#[test]
fn failing_arguments() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |
            |/bin/true bar
        "##,
        r##"
            |- /bin/true foo
        "##,
        &trim_margin(
            "
                |error:
                |expected: /bin/true foo
                |received: /bin/true bar
            ",
        )?,
    )?;
    Ok(())
}

#[test]
fn failing_later() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |
            |/bin/ls
            |/bin/false
        "##,
        r##"
            |- /bin/ls
            |- /bin/true
        "##,
        &trim_margin(
            "
                |error:
                |expected: /bin/true
                |received: /bin/false
            ",
        )?,
    )?;
    Ok(())
}

mod mismatch_in_number_of_commands {
    use super::*;

    #[test]
    fn more_expected_commands() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |
                |/bin/ls
            "##,
            r##"
                |- /bin/ls
                |- /bin/true
            "##,
            &trim_margin(
                "
                    |error:
                    |expected: /bin/true
                    |received: <script terminated>
                ",
            )?,
        )?;
        Ok(())
    }

    #[test]
    fn more_received_commands() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |
                |/bin/ls
                |/bin/true
            "##,
            r##"
                |- /bin/ls
            "##,
            &trim_margin(
                "
                    |error:
                    |expected: <protocol end>
                    |received: /bin/true
                ",
            )?,
        )?;
        Ok(())
    }
}
