use clap::{App, Arg, Error};
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum Args {
    ExecutableMock { executable_mock_path: PathBuf },
    CheckProtocols { script_path: PathBuf, record: bool },
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
        let matches = App::new("check-protocols")
            .arg(
                Arg::with_name("program")
                    .help("the program to test")
                    .required(true)
                    .index(1),
            )
            .get_matches_from_safe(args)?;
        Ok(Args::CheckProtocols {
            script_path: PathBuf::from(matches.value_of("program").unwrap()),
            record: false,
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
        let args = parse_args_safe(vec!["program", "file"].into_iter().map(String::from))?;
        match args {
            Args::CheckProtocols { script_path, .. } => {
                assert_eq!(script_path, PathBuf::from("file"),);
            }
            _ => {
                panic!("expected: Args::CheckProtocols");
            }
        }
        Ok(())
    }

    #[test]
    fn errors_on_additional_arguments() {
        assert!(
            parse_args_safe(vec!["program", "file", "foo"].into_iter().map(String::from)).is_err(),
        );
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
