use crate::R;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Command {
    pub executable: String,
    pub arguments: Vec<String>,
}

impl Command {
    pub fn format(&self) -> String {
        let mut words = vec![self.executable.to_string()];
        words.append(&mut self.arguments.clone());
        words.join(" ")
    }

    fn split_words(string: &str) -> R<Vec<String>> {
        use std::collections::VecDeque;

        #[derive(Debug)]
        struct Iter {
            original: String,
            input: VecDeque<char>,
            acc: String,
        };

        impl Iter {
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

            fn handle_escaped_char(&mut self) -> R<()> {
                match self.input.pop_front() {
                    None => self.parse_error("a backslash must be followed by a character")?,
                    Some('"') => self.acc.push('"'),
                    Some('n') => self.acc.push('\n'),
                    Some(' ') => self.acc.push(' '),
                    Some('\\') => self.acc.push('\\'),
                    Some(char) => {
                        self.parse_error(&format!("unknown escaped character {}", char))?
                    }
                }
                Ok(())
            }

            fn parse_word_inside_quotes(&mut self) -> R<()> {
                if !self.acc.is_empty() {
                    self.parse_error("opening quotes must be preceeded by a space")?;
                }
                loop {
                    match self.input.pop_front() {
                        None => self.parse_error("unmatched quotes")?,
                        Some('\\') => self.handle_escaped_char()?,
                        Some('"') => break,
                        Some(char) => self.acc.push(char),
                    }
                }
                match self.input.pop_front() {
                    Some(' ') => self.input.push_front(' '),
                    Some(_) => self.parse_error("closing quotes must be followed by a space")?,
                    None => {}
                }
                Ok(())
            }

            fn parse_word(&mut self) -> R<Option<String>> {
                self.skip_spaces();
                Ok(if self.input.is_empty() {
                    None
                } else {
                    self.acc = "".to_string();
                    loop {
                        match self.input.pop_front() {
                            None => break,
                            Some(' ') => break,
                            Some('"') => self.parse_word_inside_quotes()?,
                            Some('\\') => self.handle_escaped_char()?,
                            Some(char) => self.acc.push(char),
                        }
                    }
                    Some(self.acc.clone())
                })
            }

            fn parse_words(&mut self) -> R<Vec<String>> {
                let mut result = vec![];
                loop {
                    match self.parse_word()? {
                        None => break,
                        Some(word) => result.push(word),
                    }
                }
                Ok(result)
            }
        }

        Ok(Iter {
            original: string.to_string(),
            input: string.chars().collect(),
            acc: "".to_string(),
        }
        .parse_words()?)
    }

    pub fn new(string: &str) -> R<Command> {
        let mut words = Command::split_words(string)?.into_iter();
        let executable = match words.next() {
            None => Err(format!(
                "expected: space-separated command and arguments, got: {:?}",
                string
            ))?,
            Some(executable) => executable,
        };
        Ok(Command {
            executable,
            arguments: words.map(String::from).collect(),
        })
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
    fn quotes_next_to_letters() -> R<()> {
        assert_error!(
            Command::new(r#"foo"bar""#),
            r#"opening quotes must be preceeded by a space ("foo\"bar\"")"#
        );
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
