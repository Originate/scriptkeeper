use crate::R;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum Args {
    ExecutableMock { executable_mock_path: PathBuf },
    CheckProtocols { script_path: PathBuf },
}

pub fn parse_args(mut args: impl Iterator<Item = String>) -> R<Args> {
    args.next()
        .ok_or("argv: expected program name as argument 0")?;
    Ok(match args.next().ok_or("supply one argument")?.as_ref() {
        "--executable-mock" => Args::ExecutableMock {
            executable_mock_path: PathBuf::from(
                args.next().expect("expected executable file as argument 1"),
            ),
        },
        argument => Args::CheckProtocols {
            script_path: PathBuf::from(argument),
        },
    })
}

#[cfg(test)]
mod parse_args {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn returns_the_given_script() -> R<()> {
        assert_eq!(
            parse_args(vec!["program", "file"].into_iter().map(String::from))?,
            Args::CheckProtocols {
                script_path: PathBuf::from("file")
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
                parse_args(
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
                parse_args(
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
