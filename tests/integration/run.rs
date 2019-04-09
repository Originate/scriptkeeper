#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

use crate::utils::{prepare_script, test_run, test_run_with_tempfile, Expect};
use scriptkeeper::utils::path_to_string;
use scriptkeeper::{context::Context, run_scriptkeeper, R};
use std::env;
use std::fs;
use std::path::PathBuf;
use test_utils::{assert_error, trim_margin, TempFile};

#[test]
fn simple() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |cp
        ",
        r"
            |steps:
            |  - cp
        ",
        Expect::tests_pass(),
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
                |tests:
                |  - steps:
                |      - /usr/bin/touch {}
            ",
            path_to_string(&testfile.path())?,
        ),
    )?;
    let context = Context::new_mock();
    run_scriptkeeper(&context, &script.path())?;
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
                |tests:
                |  - steps:
                |      - {}
            ",
            &long_command_path
        ),
    )?;
    let context = Context::new_mock();
    run_scriptkeeper(&context, &script.path())?;
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
            |tests:
            |  - steps:
            |    - "true"
            |interpreter: /usr/bin/ruby
        "#,
        Expect::tests_pass(),
    )?;
    Ok(())
}

#[test]
fn allows_to_match_command_with_regex() -> R<()> {
    test_run(
        r"
            |#!/usr/bin/env bash
            |cp 1
        ",
        r#"
            |tests:
            |  - steps:
            |    - regex: cp \d
        "#,
        Expect::tests_pass(),
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
            &Context::new_mock(),
            &script,
            r"
                |steps: 42
            ",
        );
        assert_error!(
            result,
            format!(
                "error in {}.test.yaml: \
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
                |steps: - boo
            ",
        );
        assert_error!(
            result,
            format!(
                "invalid YAML in {}.test.yaml: \
                 block sequence entries are not allowed \
                 in this context at line 1 column 8",
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
            |steps:
            |  - cp
            |  - ls
        ",
        Expect::tests_pass(),
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
            |steps:
            |  - cp
        ",
        Expect::error_message(
            "
                |error:
                |  expected: cp
                |  received: mv
            ",
        )?,
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
            |steps:
            |  - ls
            |  - cp
        ",
        Expect::error_message(
            "
                |error:
                |  expected: cp
                |  received: mv
            ",
        )?,
    )?;
    Ok(())
}

mod nice_user_errors {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn nice_error_when_script_does_not_exist() {
        assert_error!(
            run_scriptkeeper(&Context::new_mock(), &PathBuf::from("./does-not-exist")),
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
                |steps:
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
                |steps:
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
                |tests:
                |  - steps:
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
                |steps:
                |  - cp foo
            ",
            Expect::tests_pass(),
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
                |steps:
                |  - cp foo
            ",
            Expect::error_message(
                "
                    |error:
                    |  expected: cp foo
                    |  received: cp bar
                ",
            )?,
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
                |steps:
                |  - cp "foo bar"
            "#,
            Expect::tests_pass(),
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
                |steps:
                |  - cp "foo bar"
            "#,
            Expect::error_message(
                r#"
                    |error:
                    |  expected: cp "foo bar"
                    |  received: cp foo bar
                "#,
            )?,
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
            |steps:
            |  - cp first
            |  - cp second
        ",
        Expect::error_message(
            "
                |error:
                |  expected: cp first
                |  received: mv first
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
            r"
                |#!/usr/bin/env bash
                |ls
            ",
            r"
                |steps:
                |  - ls
                |  - cp
            ",
            Expect::error_message(
                "
                    |error:
                    |  expected: cp
                    |  received: <script terminated>
                ",
            )?,
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
                |steps:
                |  - ls
            ",
            Expect::error_message(
                "
                    |error:
                    |  expected: <script termination>
                    |  received: cp
                ",
            )?,
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
                |steps:
                |  - command: cp
                |    stdout: test_output
                |  - cp test_output
            ",
            Expect::tests_pass(),
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
                |steps:
                |  - command: cp
                |    stdout: 'foo"'
                |  - 'cp foo\"'
            "#,
            Expect::tests_pass(),
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
                |steps:
                |  - command: cp
                |    stdout: "foo\nbar"
                |  - 'cp foo\nbar'
            "#,
            Expect::tests_pass(),
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
            |steps:
            |  - cp foo
        ",
        Expect::tests_pass(),
    )?;
    Ok(())
}

mod multiple_tests {
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
                |  steps:
                |    - cp foo
                |- arguments: bar
                |  steps:
                |    - cp bar
            ",
            Expect::tests_pass(),
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
                |- steps:
                |    - cp
                |- steps:
                |    - cp
            ",
            Expect::error_message(
                "
                    |error in test 1:
                    |  expected: cp
                    |  received: mv
                    |error in test 2:
                    |  expected: cp
                    |  received: mv
                ",
            )?,
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
                |- steps:
                |    - mv
                |- steps:
                |    - cp
            ",
            Expect::error_message(
                "
                    |test 1:
                    |  Tests passed.
                    |error in test 2:
                    |  expected: cp
                    |  received: mv
                ",
            )?,
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
                |steps:
                |  - cp test-env-var
            ",
            Expect::tests_pass(),
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
                |steps:
                |  - cp
            ",
            Expect::tests_pass(),
        )?;
        Ok(())
    }

    mod path {
        use super::*;
        use quale::which;
        use tempdir::TempDir;
        use test_utils::with_env;

        #[test]
        fn use_consistent_path_between_tracer_and_tracee() -> R<()> {
            let tempdir = TempDir::new("test")?;
            let cp_location = which("cp").ok_or("cp not found")?;
            fs::copy(&cp_location, tempdir.path().join("cp"))?;
            let path_value = format!(
                "{}:{}",
                path_to_string(tempdir.path())?,
                path_to_string(cp_location.parent().ok_or("cp location has no parent")?)?
            );
            println!("{:?}", cp_location);
            with_env("PATH", &path_value, || -> R<()> {
                test_run(
                    r"
                        |#!/usr/bin/env bash
                        |cp
                    ",
                    &format!(
                        r"
                            |steps:
                            |  - {}
                        ",
                        path_to_string(tempdir.path())?
                    ),
                    Expect::tests_pass(),
                )
            })
        }

        #[test]
        fn allows_to_overwrite_the_path() -> R<()> {
            let tempdir = TempDir::new("test")?;
            let cp_location = which("cp").ok_or("cp not found")?;
            fs::copy(&cp_location, tempdir.path().join("cp"))?;
            let path_value = format!(
                "{}:{}",
                path_to_string(tempdir.path())?,
                path_to_string(cp_location.parent().ok_or("cp location has no parent")?)?
            );
            test_run(
                r"
                    |#!/usr/bin/env bash
                    |cp
                ",
                &format!(
                    r"
                        |env:
                        |  PATH: {}
                        |steps:
                        |  - cp
                    ",
                    path_value
                ),
                Expect::tests_pass(),
            )?;
            Ok(())
        }

        #[test]
        fn separates_the_path_per_test() {}
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
            |steps:
            |  - ls
        ",
        Expect::tests_pass(),
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
                |steps:
                |  - command: grep foo
                |    exitcode: 1
                |  - ls
            ",
            Expect::tests_pass(),
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
                |steps:
                |  - command: grep foo
                |    exitcode: 0
                |  - ls
            ",
            Expect::tests_pass(),
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
                |steps:
                |  - grep foo
                |  - ls
            ",
            Expect::tests_pass(),
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
                |steps:
                |  - command: grep foo
                |    exitcode: 42
                |  - ls 42
            ",
            Expect::tests_pass(),
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
                |steps:
                |  - ls /foo/file
            ",
            Expect::tests_pass(),
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
                |steps:
                |  - ls /foo/bar/baz/foo/bar/baz/foo/bar/baz/foo/file
            ",
            Expect::tests_pass(),
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
                    |steps:
                    |  - ls {}/foo
                ",
                path_to_string(&cwd)?
            ),
            Expect::tests_pass(),
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
                |steps: []
            ",
            Expect::error_message(
                "
                    |error:
                    |  expected: <exitcode 0>
                    |  received: <exitcode 42>
                ",
            )?,
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
                |steps: []
                |exitcode: 42
            ",
            Expect::tests_pass(),
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
                |steps: []
                |exitcode: 42
            ",
            Expect::error_message(
                "
                |error:
                |  expected: <exitcode 42>
                |  received: <exitcode 0>
            ",
            )?,
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
                |tests:
                |  - steps:
                |    - ls dir
                |unmockedCommands:
                |  - dirname
            ",
            Expect::tests_pass(),
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
                |tests:
                |  - steps:
                |    - dirname dir/file
                |    - ls dir
                |unmockedCommands:
                |  - dirname
            ",
            Expect::error_message(
                "
                    |error:
                    |  expected: dirname dir/file
                    |  received: ls dir
                ",
            )?,
        )?;
        Ok(())
    }
}
