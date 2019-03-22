#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use check_protocols::utils::path_to_string;
use check_protocols::{context::Context, run_check_protocols, R};
use std::env;
use std::fs;
use std::path::PathBuf;
use test_utils::{assert_error, trim_margin, TempFile};
use utils::{prepare_script, test_run, test_run_with_tempfile};

#[test]
fn simple() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |cp
        ",
        r"
            |protocol:
            |  - cp
        ",
        Ok(()),
    )?;
    Ok(())
}

#[test]
fn does_not_execute_the_commands() -> R<()> {
    let testfile = TempFile::new()?;
    let (script, _) = prepare_script(
        &format!(
            "
                |#!/usr/bin/env bash
                |touch {}
            ",
            testfile.path().to_string_lossy()
        ),
        &format!(
            "
                |protocols:
                |  - protocol:
                |      - /usr/bin/touch {}
            ",
            path_to_string(&testfile.path())?,
        ),
    )?;
    let context = Context::new_mock();
    run_check_protocols(&context, &script.path())?;
    assert_eq!(context.get_captured_stdout(), "All tests passed.\n");
    assert!(!testfile.path().exists(), "touch was executed");
    Ok(())
}

#[test]
fn works_for_longer_file_names() -> R<()> {
    let long_command = TempFile::new()?;
    let long_command_path = long_command.path().to_string_lossy().to_string();
    fs::copy("/bin/true", &long_command_path)?;
    let (script, _) = prepare_script(
        &format!(
            r"
                |#!/usr/bin/env bash
                |{}
            ",
            &long_command_path
        ),
        &format!(
            "
                |protocols:
                |  - protocol:
                |      - {}
            ",
            &long_command_path
        ),
    )?;
    let context = Context::new_mock();
    run_check_protocols(&context, &script.path())?;
    assert_eq!(context.get_captured_stdout(), "All tests passed.\n");
    Ok(())
}

#[test]
fn can_specify_interpreter() -> R<()> {
    test_run(
        r"
            |`true`
        ",
        r#"
            |protocols:
            |  - protocol:
            |    - "true"
            |interpreter: /usr/bin/ruby
        "#,
        Ok(()),
    )?;
    Ok(())
}

mod regex_commands {
    use super::*;

    #[test]
    fn allows_to_match_command_arguments() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |cp foo bar
            ",
            r#"
                |protocols:
                |  - protocol:
                |    - regex: cp foo \w+
            "#,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn allows_to_match_command_executable() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |cp foo bar
            ",
            r#"
                |protocols:
                |  - protocol:
                |    - regex: \w+ foo bar
            "#,
            Ok(()),
        )?;
        Ok(())
    }
}

mod yaml_parse_errors {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn wrong_types() -> R<()> {
        let script = TempFile::write_temp_script(b"")?;
        let result = test_run_with_tempfile(
            &Context::new_mock(),
            &script,
            r"
                |protocol: 42
            ",
        );
        assert_error!(
            result,
            format!(
                "error in {}.protocols.yaml: \
                 expected: array, got: Integer(42)",
                path_to_string(&script.path())?
            )
        );
        Ok(())
    }

    #[test]
    fn invalid_yaml() -> R<()> {
        let script = TempFile::write_temp_script(b"")?;
        let result = test_run_with_tempfile(
            &Context::new_mock(),
            &script,
            r"
                |protocol: - boo
            ",
        );
        assert_error!(
            result,
            format!(
                "invalid YAML in {}.protocols.yaml: \
                 block sequence entries are not allowed \
                 in this context at line 1 column 11",
                path_to_string(&script.path())?
            )
        );
        Ok(())
    }
}

#[test]
fn multiple() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |cp
            |ls > /dev/null
        ",
        r"
            |protocol:
            |  - cp
            |  - ls
        ",
        Ok(()),
    )?;
    Ok(())
}

#[test]
fn failing() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |mv
        ",
        r"
            |protocol:
            |  - cp
        ",
        Err(&trim_margin(
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
fn failing_later() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |ls
            |mv
        ",
        r"
            |protocol:
            |  - ls
            |  - cp
        ",
        Err(&trim_margin(
            "
                |error:
                |  expected: cp
                |  received: mv
            ",
        )?),
    )?;
    Ok(())
}

mod nice_user_errors {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn nice_error_when_script_does_not_exist() {
        assert_error!(
            run_check_protocols(&Context::new_mock(), &PathBuf::from("./does-not-exist")),
            "executable file not found: ./does-not-exist"
        );
    }

    #[test]
    fn nice_error_when_hashbang_refers_to_missing_interpreter() -> R<()> {
        let script = TempFile::write_temp_script(
            trim_margin(
                r"
                    |#!/usr/bin/foo
                    |cp
                ",
            )?
            .as_bytes(),
        )?;
        let result = test_run_with_tempfile(
            &Context::new_mock(),
            &script,
            r"
                |protocol:
                |  - cp
            ",
        );
        assert_error!(
            result,
            trim_margin(
                format!(
                    r"
                        |execve'ing {} failed with error: ENOENT: No such file or directory
                        |Does #!/usr/bin/foo exist?
                    ",
                    path_to_string(script.path().as_ref())?
                )
                .as_str(),
            )?
            .trim()
        );
        Ok(())
    }

    #[test]
    fn nice_error_when_hashbang_missing() -> R<()> {
        let script = TempFile::write_temp_script(
            trim_margin(
                r"
                    |cp
                ",
            )?
            .as_bytes(),
        )?;
        let result = test_run_with_tempfile(
            &Context::new_mock(),
            &script,
            r"
                |protocol:
                |  - cp
            ",
        );
        assert_error!(
            result,
            trim_margin(
                format!(
                    r"
                        |execve'ing {} failed with error: ENOEXEC: Exec format error
                        |Does your interpreter exist?
                    ",
                    path_to_string(script.path().as_ref())?
                )
                .as_str(),
            )?
            .trim()
        );
        Ok(())
    }

    #[test]
    fn nice_error_when_yaml_refers_to_missing_interpreter() -> R<()> {
        let script = TempFile::write_temp_script(
            trim_margin(
                r"
                    |`true`
                ",
            )?
            .as_bytes(),
        )?;
        let result = test_run_with_tempfile(
            &Context::new_mock(),
            &script,
            r"
                |protocols:
                |  - protocol:
                |    - cp
                |interpreter: /usr/bin/foo
            ",
        );
        assert_error!(
            result,
            trim_margin(
                format!(
                    r"
                        |execve'ing {} failed with error: ENOENT: No such file or directory
                        |Does /usr/bin/foo exist?
                    ",
                    path_to_string(script.path().as_ref())?
                )
                .as_str(),
            )?
            .trim()
        );
        Ok(())
    }
}

mod arguments {
    use super::*;

    #[test]
    fn arguments() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |cp foo
            ",
            r"
                |protocol:
                |  - cp foo
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn failing_arguments() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |cp bar
            ",
            r"
                |protocol:
                |  - cp foo
            ",
            Err(&trim_margin(
                "
                    |error:
                    |  expected: cp foo
                    |  received: cp bar
                ",
            )?),
        )?;
        Ok(())
    }

    #[test]
    fn arguments_with_spaces() -> R<()> {
        test_run(
            r#"
                |#!/usr/bin/env bash
                |cp "foo bar"
            "#,
            r#"
                |protocol:
                |  - cp "foo bar"
            "#,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn error_messages_maintain_quotes() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |cp foo bar
            ",
            r#"
                |protocol:
                |  - cp "foo bar"
            "#,
            Err(&trim_margin(
                r#"
                    |error:
                    |  expected: cp "foo bar"
                    |  received: cp foo bar
                "#,
            )?),
        )?;
        Ok(())
    }
}

#[test]
fn reports_the_first_error() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |mv first
            |mv second
        ",
        r"
            |protocol:
            |  - cp first
            |  - cp second
        ",
        Err(&trim_margin(
            "
                |error:
                |  expected: cp first
                |  received: mv first
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
            r"
                |#!/usr/bin/env bash
                |ls
            ",
            r"
                |protocol:
                |  - ls
                |  - cp
            ",
            Err(&trim_margin(
                "
                    |error:
                    |  expected: cp
                    |  received: <script terminated>
                ",
            )?),
        )?;
        Ok(())
    }

    #[test]
    fn more_received_commands() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |ls
                |cp
            ",
            r"
                |protocol:
                |  - ls
            ",
            Err(&trim_margin(
                "
                    |error:
                    |  expected: <protocol end>
                    |  received: cp
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
            r"
                |#!/usr/bin/env bash
                |output=$(cp)
                |cp $output
            ",
            r"
                |protocol:
                |  - command: cp
                |    stdout: test_output
                |  - cp test_output
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn mock_stdout_with_special_characters() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |output=$(cp)
                |cp $output
            ",
            r#"
                |protocol:
                |  - command: cp
                |    stdout: 'foo"'
                |  - 'cp foo\"'
            "#,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn mock_stdout_with_newlines() -> R<()> {
        test_run(
            r#"
                |#!/usr/bin/env bash
                |output=$(cp)
                |cp "$output"
            "#,
            r#"
                |protocol:
                |  - command: cp
                |    stdout: "foo\nbar"
                |  - 'cp foo\nbar'
            "#,
            Ok(()),
        )?;
        Ok(())
    }
}

#[test]
fn pass_arguments_into_tested_script() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |cp $1
        ",
        r"
            |arguments: foo
            |protocol:
            |  - cp foo
        ",
        Ok(()),
    )?;
    Ok(())
}

mod multiple_protocols {
    use super::*;

    #[test]
    fn all_pass() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |cp $1
            ",
            r"
                |- arguments: foo
                |  protocol:
                |    - cp foo
                |- arguments: bar
                |  protocol:
                |    - cp bar
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn all_fail() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |mv
            ",
            r"
                |- protocol:
                |    - cp
                |- protocol:
                |    - cp
            ",
            Err(&trim_margin(
                "
                    |error in protocol 1:
                    |  expected: cp
                    |  received: mv
                    |error in protocol 2:
                    |  expected: cp
                    |  received: mv
                ",
            )?),
        )?;
        Ok(())
    }

    #[test]
    fn some_fail() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |mv
            ",
            r"
                |- protocol:
                |    - mv
                |- protocol:
                |    - cp
            ",
            Err(&trim_margin(
                "
                    |protocol 1:
                    |  Tests passed.
                    |error in protocol 2:
                    |  expected: cp
                    |  received: mv
                ",
            )?),
        )?;
        Ok(())
    }
}

mod environment {
    use super::*;

    #[test]
    fn pass_env_into_tested_script() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |cp $FOO
            ",
            r"
                |env:
                |  FOO: test-env-var
                |protocol:
                |  - cp test-env-var
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn does_not_inherit_the_parent_env() -> R<()> {
        std::env::set_var("FOO", "bar");
        test_run(
            r"
                |#!/usr/bin/env bash
                |cp $FOO
            ",
            r"
                |protocol:
                |  - cp
            ",
            Ok(()),
        )?;
        Ok(())
    }
}

#[test]
fn detects_running_commands_from_ruby_scripts() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env ruby
            |`ls`
        ",
        r"
            |protocol:
            |  - ls
        ",
        Ok(()),
    )?;
    Ok(())
}

mod mocked_exitcodes {
    use super::*;

    #[test]
    fn set_a_non_zero_exitcode() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |if !(grep foo) ; then
                |  ls
                |fi
            ",
            r"
                |protocol:
                |  - command: grep foo
                |    exitcode: 1
                |  - ls
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn set_a_zero_exitcode() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |if grep foo ; then
                |  ls
                |fi
            ",
            r"
                |protocol:
                |  - command: grep foo
                |    exitcode: 0
                |  - ls
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn uses_a_zero_exitcode_by_default() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |if grep foo ; then
                |  ls
                |fi
            ",
            r"
                |protocol:
                |  - grep foo
                |  - ls
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn allow_to_specify_the_exact_exitcode() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |grep foo
                |ls $?
            ",
            r"
                |protocol:
                |  - command: grep foo
                |    exitcode: 42
                |  - ls 42
            ",
            Ok(()),
        )?;
        Ok(())
    }
}

mod working_directory {
    use super::*;

    #[test]
    fn allows_to_specify_the_working_directory() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |ls $(pwd)/file
            ",
            r"
                |cwd: /foo
                |protocol:
                |  - ls /foo/file
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn works_for_long_paths() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |ls $(pwd)/file
            ",
            r"
                |cwd: /foo/bar/baz/foo/bar/baz/foo/bar/baz/foo
                |protocol:
                |  - ls /foo/bar/baz/foo/bar/baz/foo/bar/baz/foo/file
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn inherits_the_working_directory_if_not_specified() -> R<()> {
        let cwd = env::current_dir()?;
        test_run(
            r"
                |#!/usr/bin/env bash
                |ls $(pwd)/foo
            ",
            &format!(
                r"
                    |protocol:
                    |  - ls {}/foo
                ",
                path_to_string(&cwd)?
            ),
            Ok(()),
        )?;
        Ok(())
    }
}

mod expected_exitcode {
    use super::*;

    #[test]
    fn failure_when_the_tested_script_exits_with_a_non_zero_exitcode() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |exit 42
            ",
            r"
                |protocol: []
            ",
            Err(&trim_margin(
                "
                    |error:
                    |  expected: <exitcode 0>
                    |  received: <exitcode 42>
                ",
            )?),
        )?;
        Ok(())
    }

    #[test]
    fn expect_non_zero_exitcode_passing() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |exit 42
            ",
            r"
                |protocol: []
                |exitcode: 42
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn expect_non_zero_exitcode_failing() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |true
            ",
            r"
                |protocol: []
                |exitcode: 42
            ",
            Err(&trim_margin(
                "
                |error:
                |  expected: <exitcode 42>
                |  received: <exitcode 0>
            ",
            )?),
        )?;
        Ok(())
    }
}

mod unmocked_commands {
    use super::*;

    #[test]
    fn allows_to_unmock_commands() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |ls $(dirname dir/file)
            ",
            r"
                |protocols:
                |  - protocol:
                |    - ls dir
                |unmockedCommands:
                |  - dirname
            ",
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn complains_when_expected_an_unmocked_command() -> R<()> {
        test_run(
            r"
                |#!/usr/bin/env bash
                |ls $(dirname dir/file)
            ",
            r"
                |protocols:
                |  - protocol:
                |    - dirname dir/file
                |    - ls dir
                |unmockedCommands:
                |  - dirname
            ",
            Err(&trim_margin(
                "
                    |error:
                    |  expected: dirname dir/file
                    |  received: ls dir
                ",
            )?),
        )?;
        Ok(())
    }
}

mod file_mocking {
    use super::*;

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
}
