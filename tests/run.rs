use check_protocols::{run_check_protocols, Context, ExitCode, R};
use std::fs;
use std::path::PathBuf;
use test_utils::{trim_margin, TempFile};

fn test_run(script_code: &str, protocol: &str, expected: Result<(), &str>) -> R<()> {
    let script = TempFile::write_temp_script(&trim_margin(script_code)?)?;
    fs::write(
        script.path().with_extension("protocols.yaml"),
        trim_margin(protocol)?,
    )?;
    let output = run_check_protocols(Context::new_test_context(), &script.path())?;
    let expected_output = match expected {
        Err(expected_output) => (ExitCode(1), expected_output.to_string()),
        Ok(()) => (ExitCode(0), "All tests passed.\n".to_string()),
    };
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
        Ok(()),
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
        Ok(()),
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
        Ok(()),
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
        Err(&trim_margin(
            "
                |error:
                |  expected: /bin/true
                |  received: /bin/false
            ",
        )?),
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
        Err(&trim_margin(
            "
                |error:
                |  expected: /bin/true foo
                |  received: /bin/true bar
            ",
        )?),
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
        Err(&trim_margin(
            "
                |error:
                |  expected: /bin/true
                |  received: /bin/false
            ",
        )?),
    )?;
    Ok(())
}

#[test]
fn reports_the_first_error() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |
            |/bin/false first
            |/bin/false second
        "##,
        r##"
            |- /bin/true first
            |- /bin/true second
        "##,
        Err(&trim_margin(
            "
                |error:
                |  expected: /bin/true first
                |  received: /bin/false first
            ",
        )?),
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
            Err(&trim_margin(
                "
                    |error:
                    |  expected: /bin/true
                    |  received: <script terminated>
                ",
            )?),
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
            Err(&trim_margin(
                "
                    |error:
                    |  expected: <protocol end>
                    |  received: /bin/true
                ",
            )?),
        )?;
        Ok(())
    }
}

mod stdout {
    use super::*;

    #[test]
    fn mock_stdout() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |
                |output=$(/bin/true)
                |/bin/true $output
            "##,
            r##"
                |- command: /bin/true
                |  stdout: test_output
                |- /bin/true test_output
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn mock_stdout_with_special_characters() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |
                |output=$(/bin/true)
                |/bin/true $output
            "##,
            r##"
                |- command: /bin/true
                |  stdout: 'foo"'
                |- '/bin/true foo"'
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn mock_stdout_with_newlines() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |
                |output=$(/bin/true)
                |/bin/true $output
            "##,
            r##"
                |- command: /bin/true
                |  stdout: 'foo\nbar'
                |- '/bin/true foo\nbar'
            "##,
            Ok(()),
        )?;
        Ok(())
    }
}

#[test]
fn pass_arguments_into_tested_script() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |
            |/bin/true $1
        "##,
        r##"
            |arguments: foo
            |protocol:
            |  - '/bin/true foo'
        "##,
        Ok(()),
    )?;
    Ok(())
}

mod multiple_protocols {
    use super::*;

    #[test]
    fn all_pass() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |
                |/bin/true $1
            "##,
            r##"
                |arguments: foo
                |protocol:
                |  - '/bin/true foo'
                |---
                |arguments: bar
                |protocol:
                |  - '/bin/true bar'
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn all_fail() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |
                |/bin/false
            "##,
            r##"
                |- /bin/true
                |---
                |- /bin/true
            "##,
            Err(&trim_margin(
                "
                    |error in protocol 1:
                    |  expected: /bin/true
                    |  received: /bin/false
                    |error in protocol 2:
                    |  expected: /bin/true
                    |  received: /bin/false
                ",
            )?),
        )?;
        Ok(())
    }

    #[test]
    fn some_fail() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |
                |/bin/false
            "##,
            r##"
                |- /bin/false
                |---
                |- /bin/true
            "##,
            Err(&trim_margin(
                "
                    |protocol 1:
                    |  Tests passed.
                    |error in protocol 2:
                    |  expected: /bin/true
                    |  received: /bin/false
                ",
            )?),
        )?;
        Ok(())
    }
}

#[test]
fn nice_error_when_script_does_not_exist() {
    let result = run_check_protocols(
        Context::new_test_context(),
        &PathBuf::from("./does-not-exist"),
    );
    assert_eq!(
        format!("{}", result.unwrap_err()),
        "executable file not found: ./does-not-exist"
    );
}
