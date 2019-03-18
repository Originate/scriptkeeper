use super::argument_parser::Parser;
use super::executable_path;
use crate::R;
use std::str;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Command {
    pub executable: Vec<u8>,
    pub arguments: Vec<Vec<u8>>,
}

impl Command {
    fn add_quotes_if_needed(word: String) -> String {
        if word.chars().any(|char| char == ' ') {
            format!("\"{}\"", word)
        } else {
            word
        }
    }

    pub fn compare(&self, other: &Command) -> bool {
        executable_path::compare_executables(&self.executable, &other.executable)
            && self.arguments == other.arguments
    }

    fn escape(word: String) -> String {
        fn escape_char(char: char) -> String {
            match char {
                '"' => "\\\"".to_string(),
                '\n' => "\\n".to_string(),
                '\\' => "\\\\".to_string(),
                _ => char.to_string(),
            }
        }
        word.chars().map(escape_char).collect::<Vec<_>>().join("")
    }

    pub fn format(&self) -> String {
        let mut words =
            vec![
                String::from_utf8_lossy(&executable_path::canonicalize(&self.executable))
                    .into_owned(),
            ];
        words.append(
            &mut self
                .arguments
                .clone()
                .into_iter()
                .map(|argument| Command::escape(String::from_utf8_lossy(&argument).to_string()))
                .map(Command::add_quotes_if_needed)
                .collect(),
        );
        words.join(" ")
    }

    pub fn new(command: &str) -> R<Command> {
        let mut words = Parser::parse_arguments(command)?
            .into_iter()
            .map(|word| word.into_bytes());
        match words.next() {
            Some(executable) => Ok(Command {
                executable,
                arguments: words.collect(),
            }),
            None => Err(format!(
                "expected: space-separated command and arguments ({:?})",
                command.to_string()
            ))?,
        }
    }
}

#[cfg(test)]
mod command {
    use super::*;

    mod new {
        use super::Command;
        use super::*;
        use test_utils::{assert_error, Mappable};

        #[test]
        fn splits_words() -> R<()> {
            assert_eq!(
                Command::new("foo bar")?,
                Command {
                    executable: b"foo".to_vec(),
                    arguments: vec![b"bar".to_vec()]
                }
            );
            Ok(())
        }

        #[test]
        fn honors_quotes() -> R<()> {
            assert_eq!(
                Command::new(r#"foo "bar baz""#)?,
                Command {
                    executable: b"foo".to_vec(),
                    arguments: vec![b"bar baz".to_vec()]
                }
            );
            Ok(())
        }

        #[test]
        fn honors_escaped_quotes_outside_quotes() -> R<()> {
            assert_eq!(
                Command::new(r#"foo\" bar baz"#)?,
                Command {
                    executable: b"foo\"".to_vec(),
                    arguments: vec![b"bar", b"baz"].map(|arg| arg.to_vec())
                }
            );
            assert_eq!(
                Command::new(r#"foo\" "bar baz""#)?,
                Command {
                    executable: b"foo\"".to_vec(),
                    arguments: vec![b"bar baz".to_vec()]
                }
            );
            Ok(())
        }

        #[test]
        fn honors_escaped_quotes_inside_quotes() -> R<()> {
            assert_eq!(
                Command::new(r#"foo "bar\" baz""#)?,
                Command {
                    executable: b"foo".to_vec(),
                    arguments: vec![b"bar\" baz".to_vec()]
                }
            );
            Ok(())
        }

        #[test]
        fn quotes_next_to_letters_1() -> R<()> {
            assert_error!(
                Command::new(r#"foo"bar""#),
                r#"opening quotes must be preceeded by a space ("foo\"bar\"")"#
            );
            Ok(())
        }

        #[test]
        fn quotes_next_to_letters_2() -> R<()> {
            assert_error!(
                Command::new(r#""foo"bar"#),
                r#"closing quotes must be followed by a space ("\"foo\"bar")"#
            );
            Ok(())
        }

        #[test]
        fn nonmatching_quotes() -> R<()> {
            assert_error!(
                Command::new(r#"foo "bar"#),
                r#"unmatched quotes ("foo \"bar")"#
            );
            Ok(())
        }

        #[test]
        fn double_spaces() -> R<()> {
            assert_eq!(
                Command::new("foo  bar")?,
                Command {
                    executable: b"foo".to_vec(),
                    arguments: vec![b"bar".to_vec()]
                }
            );
            Ok(())
        }

        #[test]
        fn leading_spaces() -> R<()> {
            assert_eq!(
                Command::new(" foo bar")?,
                Command {
                    executable: b"foo".to_vec(),
                    arguments: vec![b"bar".to_vec()]
                }
            );
            Ok(())
        }

        #[test]
        fn trailing_spaces() -> R<()> {
            assert_eq!(
                Command::new("foo bar ")?,
                Command {
                    executable: b"foo".to_vec(),
                    arguments: vec![b"bar".to_vec()]
                }
            );
            Ok(())
        }

        mod escaping {
            use super::*;

            #[test]
            fn newlines() -> R<()> {
                assert_eq!(
                    Command::new(r#"foo "bar\nbaz""#)?,
                    Command {
                        executable: b"foo".to_vec(),
                        arguments: vec![b"bar\nbaz".to_vec()]
                    }
                );
                Ok(())
            }

            #[test]
            fn escaping_spaces() -> R<()> {
                assert_eq!(
                    Command::new(r#"foo bar\ baz"#)?,
                    Command {
                        executable: b"foo".to_vec(),
                        arguments: vec![b"bar baz".to_vec()]
                    }
                );
                Ok(())
            }

            #[test]
            fn escaping_backslashes() -> R<()> {
                assert_eq!(
                    Command::new(r#"foo bar\\baz"#)?,
                    Command {
                        executable: b"foo".to_vec(),
                        arguments: vec![br#"bar\baz"#.to_vec()]
                    }
                );
                Ok(())
            }
        }
    }

    mod format {
        use super::*;

        macro_rules! roundtrip {
            ($name:ident, $string:expr) => {
                #[test]
                fn $name() -> R<()> {
                    assert_eq!(Command::new($string)?.format(), $string);
                    Ok(())
                }
            };
        }

        macro_rules! normalizing_roundtrip {
            ($name:ident, $input:expr, $normalized:expr) => {
                #[test]
                fn $name() -> R<()> {
                    assert_eq!(Command::new($input)?.format(), $normalized);
                    Ok(())
                }
            };
        }

        roundtrip!(simple_command, "foo");

        roundtrip!(command_and_arguments, "foo bar baz");

        roundtrip!(quoted_argument_with_space, r##"foo "bar baz""##);

        normalizing_roundtrip!(quoted_argument_without_space, r##"foo "bar""##, "foo bar");

        roundtrip!(escaped_quotes, r##"foo bar\""##);

        normalizing_roundtrip!(
            escaped_quotes_in_quotes,
            r##"foo "bar\"""##,
            r##"foo bar\""##
        );

        normalizing_roundtrip!(
            puts_escaped_space_in_quotes,
            r##"foo bar\ baz"##,
            r##"foo "bar baz""##
        );

        roundtrip!(escaped_newlines, r##"foo bar\nbaz"##);

        roundtrip!(backslash, r##"foo bar\\baz"##);
    }
}
