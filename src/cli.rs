use clap::{App, Arg, Error};
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum Args {
    ExecutableMock { executable_mock_path: PathBuf },
    Scriptkeeper { script_path: PathBuf, record: bool },
}

pub fn parse_args(args: impl Iterator<Item = String>) -> Args {
    match parse_args_safe(args) {
        Ok(args) => args,
        Err(error) => error.exit(),
    }
}

fn parse_args_safe(args: impl Iterator<Item = String>) -> Result<Args, Error> {
    let args: Vec<_> = args.collect();
    if args.get(1) == Some(&"--executable-mock".to_string()) {
        Ok(Args::ExecutableMock {
            executable_mock_path: PathBuf::from(
                args.get(2).expect("missing argument: executable mock file"),
            ),
        })
    } else {
        let matches = App::new("scriptkeeper")
            .arg(Arg::with_name("record").short("r").long("record").help(
                "[EXPERIMENTAL] Runs the script (without mocking out anything), \
                 records a test case and prints it to stdout",
            ))
            .arg(
                Arg::with_name("program")
                    .help("the program to test")
                    .required(true)
                    .index(1),
            )
            .get_matches_from_safe(args)?;
        Ok(Args::Scriptkeeper {
            script_path: PathBuf::from(matches.value_of("program").unwrap()),
            record: matches.is_present("record"),
        })
    }
}

#[cfg(test)]
mod parse_args_safe {
    use super::*;
    use crate::R;
    use pretty_assertions::assert_eq;

    #[test]
    fn returns_the_given_script() -> R<()> {
        assert_eq!(
            parse_args_safe(vec!["program", "file"].into_iter().map(String::from))?,
            Args::Scriptkeeper {
                script_path: PathBuf::from("file"),
                record: false
            }
        );
        Ok(())
    }

    #[test]
    fn errors_on_additional_arguments() {
        assert!(
            parse_args_safe(vec!["program", "file", "foo"].into_iter().map(String::from)).is_err(),
        );
    }

    #[test]
    fn respects_the_record_flag() -> R<()> {
        assert_eq!(
            parse_args_safe(
                vec!["program", "--record", "file"]
                    .into_iter()
                    .map(String::from),
            )?,
            Args::Scriptkeeper {
                script_path: PathBuf::from("file"),
                record: true
            }
        );
        Ok(())
    }

    mod executable_mock {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn returns_the_executable_mock_file() -> R<()> {
            assert_eq!(
                parse_args_safe(
                    vec!["program", "--executable-mock", "file"]
                        .into_iter()
                        .map(String::from)
                )?,
                Args::ExecutableMock {
                    executable_mock_path: PathBuf::from("file")
                }
            );
            Ok(())
        }

        #[test]
        fn allows_arbitrary_arguments() -> R<()> {
            assert_eq!(
                parse_args_safe(
                    vec!["program", "--executable-mock", "file", "foo", "bar"]
                        .into_iter()
                        .map(String::from)
                )?,
                Args::ExecutableMock {
                    executable_mock_path: PathBuf::from("file")
                }
            );
            Ok(())
        }
    }
}
