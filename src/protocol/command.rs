use super::argument_parser::Parser;
use super::executable_path;
use crate::R;
use regex::Regex;
use std::ffi::OsString;
use std::path::PathBuf;
use std::str;

#[derive(Debug, Clone)]
pub enum CommandMatcher {
    Exact(Command),
    RegexMatch(Regex),
}

impl PartialEq for CommandMatcher {
    fn eq(&self, other: &CommandMatcher) -> bool {
        use CommandMatcher::*;
        match (self, other) {
            (Exact(a), Exact(b)) => a == b,
            (RegexMatch(a), RegexMatch(b)) => a.as_str() == b.as_str(),
            _ => false,
        }
    }
}

impl Eq for CommandMatcher {}

impl CommandMatcher {
    pub fn matches_received(&self, received: &Command) -> bool {
        match self {
            CommandMatcher::Exact(command) => {
                executable_path::compare_executables(&command.executable, &received.executable)
                    && command.arguments == received.arguments
            }
            CommandMatcher::RegexMatch(regex) => regex.is_match(&received.format()),
        }
    }

    pub fn format(&self) -> String {
        match self {
            CommandMatcher::Exact(command) => command.format(),
            CommandMatcher::RegexMatch(regex) => regex.as_str().to_string(),
        }
    }

    pub fn regex_match(string: &str) -> R<CommandMatcher> {
        Ok(CommandMatcher::RegexMatch(Regex::new(&format!(
            "^{}$",
            string
        ))?))
    }

    pub fn exact_match(command: &str) -> R<CommandMatcher> {
        Ok(CommandMatcher::Exact(Command::new(command)?))
    }
}

#[cfg(test)]
mod command_matcher {
    use super::*;

    #[test]
    fn exact_command_matches_received() -> R<()> {
        assert!(CommandMatcher::exact_match("cp ./")?.matches_received(&Command::new("cp ./")?));
        Ok(())
    }

    #[test]
    fn exact_command_doesnt_match_different_command() -> R<()> {
        assert!(!CommandMatcher::exact_match("cp ./")?.matches_received(&Command::new("bar")?));
        Ok(())
    }

    #[test]
    fn regex_matches_received() -> R<()> {
        assert!(CommandMatcher::regex_match("cp \\w")?.matches_received(&Command::new("cp a")?));
        Ok(())
    }

    #[test]
    fn regex_doesnt_match_received_if_regex_doesnt_match() -> R<()> {
        assert!(!CommandMatcher::regex_match("cp \\d")?.matches_received(&Command::new("cp a")?));
        Ok(())
    }

    #[test]
    fn regex_only_matches_if_entire_command_is_a_match() -> R<()> {
        assert!(!CommandMatcher::regex_match("cp \\d")?.matches_received(&Command::new("cp 1 2")?));
        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Command {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
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

    pub fn format_arguments(arguments: Vec<OsString>) -> String {
        arguments
            .into_iter()
            .map(|argument| Command::escape(argument.to_string_lossy().into_owned()))
            .map(Command::add_quotes_if_needed)
            .collect::<Vec<String>>()
            .join(" ")
    }

    pub fn format(&self) -> String {
        let executable = executable_path::canonicalize(&self.executable)
            .to_string_lossy()
            .into_owned();
        if self.arguments.is_empty() {
            executable
        } else {
            format!(
                "{} {}",
                executable,
                Command::format_arguments(self.arguments.clone())
            )
        }
    }

    pub fn new(command: &str) -> R<Command> {
        let mut words = Parser::parse_arguments(command)?.into_iter();
        match words.next() {
            Some(executable) => Ok(Command {
                executable: PathBuf::from(executable),
                arguments: words.map(OsString::from).collect(),
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
                    executable: PathBuf::from("foo"),
                    arguments: vec![OsString::from("bar")]
                }
            );
            Ok(())
        }

        #[test]
        fn honors_quotes() -> R<()> {
            assert_eq!(
                Command::new(r#"foo "bar baz""#)?,
                Command {
                    executable: PathBuf::from("foo"),
                    arguments: vec![OsString::from("bar baz")]
                }
            );
            Ok(())
        }

        #[test]
        fn honors_escaped_quotes_outside_quotes() -> R<()> {
            assert_eq!(
                Command::new(r#"foo\" bar baz"#)?,
                Command {
                    executable: PathBuf::from("foo\""),
                    arguments: vec!["bar", "baz"].map(OsString::from)
                }
            );
            assert_eq!(
                Command::new(r#"foo\" "bar baz""#)?,
                Command {
                    executable: PathBuf::from("foo\""),
                    arguments: vec![OsString::from("bar baz")]
                }
            );
            Ok(())
        }

        #[test]
        fn honors_escaped_quotes_inside_quotes() -> R<()> {
            assert_eq!(
                Command::new(r#"foo "bar\" baz""#)?,
                Command {
                    executable: PathBuf::from("foo"),
                    arguments: vec![OsString::from("bar\" baz")]
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
                    executable: PathBuf::from("foo"),
                    arguments: vec![OsString::from("bar")]
                }
            );
            Ok(())
        }

        #[test]
        fn leading_spaces() -> R<()> {
            assert_eq!(
                Command::new(" foo bar")?,
                Command {
                    executable: PathBuf::from("foo"),
                    arguments: vec![OsString::from("bar")]
                }
            );
            Ok(())
        }

        #[test]
        fn trailing_spaces() -> R<()> {
            assert_eq!(
                Command::new("foo bar ")?,
                Command {
                    executable: PathBuf::from("foo"),
                    arguments: vec![OsString::from("bar")]
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
                        executable: PathBuf::from("foo"),
                        arguments: vec![OsString::from("bar\nbaz")]
                    }
                );
                Ok(())
            }

            #[test]
            fn escaping_spaces() -> R<()> {
                assert_eq!(
                    Command::new(r#"foo bar\ baz"#)?,
                    Command {
                        executable: PathBuf::from("foo"),
                        arguments: vec![OsString::from("bar baz")]
                    }
                );
                Ok(())
            }

            #[test]
            fn escaping_backslashes() -> R<()> {
                assert_eq!(
                    Command::new(r#"foo bar\\baz"#)?,
                    Command {
                        executable: PathBuf::from("foo"),
                        arguments: vec![OsString::from(r"bar\baz")]
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

        roundtrip!(quoted_argument_with_space, r#"foo "bar baz""#);

        normalizing_roundtrip!(quoted_argument_without_space, r#"foo "bar""#, "foo bar");

        roundtrip!(escaped_quotes, r#"foo bar\""#);

        normalizing_roundtrip!(escaped_quotes_in_quotes, r#"foo "bar\"""#, r#"foo bar\""#);

        normalizing_roundtrip!(
            puts_escaped_space_in_quotes,
            r"foo bar\ baz",
            r#"foo "bar baz""#
        );

        roundtrip!(escaped_newlines, r"foo bar\nbaz");

        roundtrip!(backslash, r"foo bar\\baz");
    }
}
