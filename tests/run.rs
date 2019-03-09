#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use check_protocols::utils::path_to_string;
use check_protocols::{run_check_protocols, Context, R};
use std::env;
use std::path::PathBuf;
use test_utils::{assert_error, trim_margin, TempFile};
use utils::{test_run, test_run_with_tempfile};

#[test]
fn simple() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |/bin/true
        "##,
        r##"
            |protocol:
            |  - /bin/true
        "##,
        Ok(()),
    )?;
    Ok(())
}

#[test]
fn can_specify_interpreter() -> R<()> {
    test_run(
        r##"
            |`true`
        "##,
        r##"
            |protocols:
            |  - protocol:
            |    - /bin/true
            |interpreter: /usr/bin/ruby
        "##,
        Ok(()),
    )?;
    Ok(())
}

mod yaml_parse_errors {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn wrong_types() -> R<()> {
        let script = TempFile::write_temp_script(b"")?;
        let result = test_run_with_tempfile(
            &script,
            r##"
                |protocol: 42
            "##,
        );
        assert_error!(
            result,
            format!(
                "unexpected type in {}.protocols.yaml: \
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
            &script,
            r##"
                |protocol: - boo
            "##,
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
        r##"
            |#!/usr/bin/env bash
            |/bin/true
            |/bin/ls > /dev/null
        "##,
        r##"
            |protocol:
            |  - /bin/true
            |  - /bin/ls
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
            |/bin/false
        "##,
        r##"
            |protocol:
            |  - /bin/true
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
fn failing_later() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |/bin/ls
            |/bin/false
        "##,
        r##"
            |protocol:
            |  - /bin/ls
            |  - /bin/true
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

mod nice_user_errors {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn nice_error_when_script_does_not_exist() {
        let result = run_check_protocols(
            Context::new_test_context(),
            &PathBuf::from("./does-not-exist"),
        );
        assert_error!(result, "executable file not found: ./does-not-exist");
    }

    #[test]
    fn nice_error_when_hashbang_refers_to_missing_interpreter() -> R<()> {
        let script = TempFile::write_temp_script(
            trim_margin(
                r##"
                    |#!/usr/bin/foo
                    |/bin/true
                "##,
            )?
            .as_bytes(),
        )?;
        let result = test_run_with_tempfile(
            &script,
            r##"
                |protocol:
                |  - /bin/true
            "##,
        );
        assert_error!(
            result,
            trim_margin(
                format!(
                    r##"
                        |execve'ing {} failed with error: ENOENT: No such file or directory
                        |Does #!/usr/bin/foo exist?
                    "##,
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
                r##"
                    |/bin/true
                "##,
            )?
            .as_bytes(),
        )?;
        let result = test_run_with_tempfile(
            &script,
            r##"
                |protocol:
                |  - /bin/true
            "##,
        );
        assert_error!(
            result,
            trim_margin(
                format!(
                    r##"
                        |execve'ing {} failed with error: ENOEXEC: Exec format error
                        |Does your interpreter exist?
                    "##,
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
                r##"
                    |`true`
                "##,
            )?
            .as_bytes(),
        )?;
        let result = test_run_with_tempfile(
            &script,
            r##"
                |protocols:
                |  - protocol:
                |    - /bin/true
                |interpreter: /usr/bin/foo
            "##,
        );
        assert_error!(
            result,
            trim_margin(
                format!(
                    r##"
                        |execve'ing {} failed with error: ENOENT: No such file or directory
                        |Does /usr/bin/foo exist?
                    "##,
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
            r##"
                |#!/usr/bin/env bash
                |/bin/true foo
            "##,
            r##"
                |protocol:
                |  - /bin/true foo
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn failing_arguments() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |/bin/true bar
            "##,
            r##"
                |protocol:
                |  - /bin/true foo
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
    fn arguments_with_spaces() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |/bin/true "foo bar"
            "##,
            r##"
                |protocol:
                |  - /bin/true "foo bar"
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn error_messages_maintain_quotes() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |/bin/true foo bar
            "##,
            r##"
                |protocol:
                |  - /bin/true "foo bar"
            "##,
            Err(&trim_margin(
                r##"
                    |error:
                    |  expected: /bin/true "foo bar"
                    |  received: /bin/true foo bar
                "##,
            )?),
        )?;
        Ok(())
    }
}

#[test]
fn reports_the_first_error() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env bash
            |/bin/false first
            |/bin/false second
        "##,
        r##"
            |protocol:
            |  - /bin/true first
            |  - /bin/true second
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
                |/bin/ls
            "##,
            r##"
                |protocol:
                |  - /bin/ls
                |  - /bin/true
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
                |/bin/ls
                |/bin/true
            "##,
            r##"
                |protocol:
                |  - /bin/ls
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
                |output=$(/bin/true)
                |/bin/true $output
            "##,
            r##"
                |protocol:
                |  - command: /bin/true
                |    stdout: test_output
                |  - /bin/true test_output
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
                |output=$(/bin/true)
                |/bin/true $output
            "##,
            r##"
                |protocol:
                |  - command: /bin/true
                |    stdout: 'foo"'
                |  - '/bin/true foo\"'
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
                |output=$(/bin/true)
                |/bin/true "$output"
            "##,
            r##"
                |protocol:
                |  - command: /bin/true
                |    stdout: "foo\nbar"
                |  - '/bin/true foo\nbar'
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
            |/bin/true $1
        "##,
        r##"
            |arguments: foo
            |protocol:
            |  - /bin/true foo
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
                |/bin/true $1
            "##,
            r##"
                |- arguments: foo
                |  protocol:
                |    - /bin/true foo
                |- arguments: bar
                |  protocol:
                |    - /bin/true bar
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
                |/bin/false
            "##,
            r##"
                |- protocol:
                |    - /bin/true
                |- protocol:
                |    - /bin/true
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
                |/bin/false
            "##,
            r##"
                |- protocol:
                |    - /bin/false
                |- protocol:
                |    - /bin/true
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

mod environment {
    use super::*;

    #[test]
    fn pass_env_into_tested_script() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |/bin/true $FOO
            "##,
            r##"
                |env:
                |  FOO: test-env-var
                |protocol:
                |  - /bin/true test-env-var
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn does_not_inherit_the_parent_env() -> R<()> {
        std::env::set_var("FOO", "BAR");
        test_run(
            r##"
                |#!/usr/bin/env bash
                |/bin/true $FOO
            "##,
            r##"
                |protocol:
                |  - /bin/true
            "##,
            Ok(()),
        )?;
        Ok(())
    }
}

#[test]
fn detects_running_commands_from_ruby_scripts() -> R<()> {
    test_run(
        r##"
            |#!/usr/bin/env ruby
            |`ls`
        "##,
        r##"
            |protocol:
            |  - /bin/ls
        "##,
        Ok(()),
    )?;
    Ok(())
}

mod mocked_exitcodes {
    use super::*;

    #[test]
    fn set_a_non_zero_exitcode() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |if !(grep foo) ; then
                |  ls
                |fi
            "##,
            r##"
                |protocol:
                |  - command: /bin/grep foo
                |    exitcode: 1
                |  - /bin/ls
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn set_a_zero_exitcode() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |if grep foo ; then
                |  ls
                |fi
            "##,
            r##"
                |protocol:
                |  - command: /bin/grep foo
                |    exitcode: 0
                |  - /bin/ls
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn uses_a_zero_exitcode_by_default() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |if grep foo ; then
                |  ls
                |fi
            "##,
            r##"
                |protocol:
                |  - /bin/grep foo
                |  - /bin/ls
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn allow_to_specify_the_exact_exitcode() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |grep foo
                |ls $?
            "##,
            r##"
                |protocol:
                |  - command: /bin/grep foo
                |    exitcode: 42
                |  - /bin/ls 42
            "##,
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
            r##"
                |#!/usr/bin/env bash
                |ls $(pwd)/file
            "##,
            r##"
                |cwd: /foo
                |protocol:
                |  - /bin/ls /foo/file
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn works_for_long_paths() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |ls $(pwd)/file
            "##,
            r##"
                |cwd: /foo/bar/baz/foo/bar/baz/foo/bar/baz/foo
                |protocol:
                |  - /bin/ls /foo/bar/baz/foo/bar/baz/foo/bar/baz/foo/file
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn inherits_the_working_directory_if_not_specified() -> R<()> {
        let cwd = env::current_dir()?;
        test_run(
            r##"
                |#!/usr/bin/env bash
                |ls $(pwd)/foo
            "##,
            &format!(
                r##"
                    |protocol:
                    |  - /bin/ls {}/foo
                "##,
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
            r##"
                |#!/usr/bin/env bash
                |exit 42
            "##,
            r##"
                |protocol: []
            "##,
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
            r##"
                |#!/usr/bin/env bash
                |exit 42
            "##,
            r##"
                |protocol: []
                |exitcode: 42
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn expect_non_zero_exitcode_failing() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |true
            "##,
            r##"
                |protocol: []
                |exitcode: 42
            "##,
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
            r##"
                |#!/usr/bin/env bash
                |ls $(dirname dir/file)
            "##,
            r##"
                |protocols:
                |  - protocol:
                |    - /bin/ls dir
                |unmockedCommands:
                |  - /usr/bin/dirname
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn complains_when_expected_an_unmocked_command() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |ls $(dirname dir/file)
            "##,
            r##"
                |protocols:
                |  - protocol:
                |    - /usr/bin/dirname dir/file
                |    - /bin/ls dir
                |unmockedCommands:
                |  - /usr/bin/dirname
            "##,
            Err(&trim_margin(
                "
                    |error:
                    |  expected: /usr/bin/dirname dir/file
                    |  received: /bin/ls dir
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
            r##"
                |#!/usr/bin/env bash
                |if [ -f /foo ]; then
                |  /bin/true
                |fi
            "##,
            r##"
                |protocols:
                |  - protocol:
                |      - /bin/true
                |    mockedFiles:
                |      - /foo
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn allows_to_mock_directory_existence() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |if [ -d /foo/ ]; then
                |  /bin/true
                |fi
            "##,
            r##"
                |protocols:
                |  - protocol:
                |      - command: /bin/true
                |    mockedFiles:
                |      - /foo/
            "##,
            Ok(()),
        )?;
        Ok(())
    }

    #[test]
    fn does_not_mock_existence_of_unspecified_files() -> R<()> {
        test_run(
            r##"
                |#!/usr/bin/env bash
                |if [ -f /foo ]; then
                |  /bin/true
                |fi
            "##,
            r##"
                |protocols:
                |  - protocol: []
            "##,
            Ok(()),
        )?;
        Ok(())
    }
}
