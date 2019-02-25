use crate::R;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Command {
    pub executable: String,
    pub arguments: Vec<String>,
}

impl Command {
    fn add_quotes_if_needed(word: String) -> String {
        if word.chars().any(|char| char == ' ') {
            format!("\"{}\"", word)
        } else {
            word
        }
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
        let mut words = vec![self.executable.to_string()];
        words.append(
            &mut self
                .arguments
                .clone()
                .into_iter()
                .map(Command::escape)
                .map(Command::add_quotes_if_needed)
                .collect(),
        );
        words.join(" ")
    }

    pub fn new(string: &str) -> R<Command> {
        use std::collections::VecDeque;

        #[derive(Debug)]
        struct Parser {
            original: String,
            input: VecDeque<char>,
        };

        impl Parser {
            fn parse_error<A>(&self, message: &str) -> R<A> {
                Err(format!("{} ({:?})", message, self.original))?
            }

            fn skip_spaces(&mut self) {
                loop {
                    match self.input.pop_front() {
                        Some(' ') => {}
                        Some(char) => {
                            self.input.push_front(char);
                            break;
                        }
                        None => break,
                    }
                }
            }

            fn skip_char(&mut self, expected: char, message: &str) -> R<()> {
                let next = self.input.pop_front();
                if next != Some(expected) {
                    self.parse_error(message)?;
                }
                Ok(())
            }

            fn peek_chars(&mut self, expected: Vec<Option<char>>, message: &str) -> R<()> {
                let peeked = self.input.get(0);
                if !expected.contains(&peeked.cloned()) {
                    self.parse_error(message)?;
                }
                Ok(())
            }

            fn parse_char(&mut self, excluded: &[char]) -> R<Option<char>> {
                Ok(match self.input.pop_front() {
                    None => None,
                    Some('\\') => Some(match self.input.pop_front() {
                        None => self.parse_error("a backslash must be followed by a character")?,
                        Some('"') => '"',
                        Some('n') => '\n',
                        Some(' ') => ' ',
                        Some('\\') => '\\',
                        Some(char) => {
                            self.parse_error(&format!("unknown escaped character {}", char))?
                        }
                    }),
                    Some(char) => {
                        if excluded.contains(&char) {
                            self.input.push_front(char);
                            None
                        } else {
                            Some(char)
                        }
                    }
                })
            }

            fn collect_chars_until(&mut self, excluded: &[char]) -> R<String> {
                let mut result = "".to_string();
                while let Some(char) = self.parse_char(excluded)? {
                    result.push(char);
                }
                Ok(result)
            }

            fn parse_word(&mut self) -> R<Option<String>> {
                self.skip_spaces();
                Ok(match self.input.get(0) {
                    None => None,
                    Some('"') => {
                        self.skip_char('"', "shouldn't happen")?;
                        let word = self.collect_chars_until(&['"'])?;
                        self.skip_char('"', "unmatched quotes")?;
                        self.peek_chars(
                            vec![Some(' '), None],
                            "closing quotes must be followed by a space",
                        )?;
                        Some(word)
                    }
                    Some(_) => {
                        let result = self.collect_chars_until(&[' ', '"'])?;
                        self.peek_chars(
                            vec![Some(' '), None],
                            "opening quotes must be preceeded by a space",
                        )?;
                        Some(result)
                    }
                })
            }

            fn parse_command(&mut self) -> R<Command> {
                let executable = match self.parse_word()? {
                    None => self.parse_error("expected: space-separated command and arguments")?,
                    Some(executable) => executable,
                };
                let mut arguments = vec![];
                loop {
                    match self.parse_word()? {
                        None => break,
                        Some(word) => arguments.push(word),
                    }
                }
                Ok(Command {
                    executable,
                    arguments,
                })
            }
        }

        Ok(Parser {
            original: string.to_string(),
            input: string.chars().collect(),
        }
        .parse_command()?)
    }

    pub fn compare(&self, other: &Command) -> Result<(), (String, String)> {
        if self != other {
            Err((self.format(), other.format()))
        } else {
            Ok(())
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
                    executable: "foo".to_string(),
                    arguments: vec!["bar".to_string()]
                }
            );
            Ok(())
        }

        #[test]
        fn honors_quotes() -> R<()> {
            assert_eq!(
                Command::new(r#"foo "bar baz""#)?,
                Command {
                    executable: "foo".to_string(),
                    arguments: vec!["bar baz".to_string()]
                }
            );
            Ok(())
        }

        #[test]
        fn honors_escaped_quotes_outside_quotes() -> R<()> {
            assert_eq!(
                Command::new(r#"foo\" bar baz"#)?,
                Command {
                    executable: "foo\"".to_string(),
                    arguments: vec!["bar", "baz"].map(String::from)
                }
            );
            assert_eq!(
                Command::new(r#"foo\" "bar baz""#)?,
                Command {
                    executable: "foo\"".to_string(),
                    arguments: vec!["bar baz".to_string()]
                }
            );
            Ok(())
        }

        #[test]
        fn honors_escaped_quotes_inside_quotes() -> R<()> {
            assert_eq!(
                Command::new(r#"foo "bar\" baz""#)?,
                Command {
                    executable: "foo".to_string(),
                    arguments: vec!["bar\" baz".to_string()]
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
                    executable: "foo".to_string(),
                    arguments: vec!["bar".to_string()]
                }
            );
            Ok(())
        }

        #[test]
        fn leading_spaces() -> R<()> {
            assert_eq!(
                Command::new(" foo bar")?,
                Command {
                    executable: "foo".to_string(),
                    arguments: vec!["bar".to_string()]
                }
            );
            Ok(())
        }

        #[test]
        fn trailing_spaces() -> R<()> {
            assert_eq!(
                Command::new("foo bar ")?,
                Command {
                    executable: "foo".to_string(),
                    arguments: vec!["bar".to_string()]
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
                        executable: "foo".to_string(),
                        arguments: vec!["bar\nbaz".to_string()]
                    }
                );
                Ok(())
            }

            #[test]
            fn escaping_spaces() -> R<()> {
                assert_eq!(
                    Command::new(r#"foo bar\ baz"#)?,
                    Command {
                        executable: "foo".to_string(),
                        arguments: vec!["bar baz".to_string()]
                    }
                );
                Ok(())
            }

            #[test]
            fn escaping_backslashes() -> R<()> {
                assert_eq!(
                    Command::new(r#"foo bar\\baz"#)?,
                    Command {
                        executable: "foo".to_string(),
                        arguments: vec![r#"bar\baz"#.to_string()]
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
