use super::command::Command;
use crate::R;
use regex::Regex;
use std::path::PathBuf;
use std::str;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CommandMatcher {
    ExactMatch(Command),
    RegexMatch(AnchoredRegex),
}

impl CommandMatcher {
    pub fn matches(&self, mocked_executables: &[PathBuf], other: &Command) -> bool {
        match self {
            CommandMatcher::ExactMatch(command) => command.compare(mocked_executables, other),
            CommandMatcher::RegexMatch(regex) => regex.is_match(&other.format(mocked_executables)),
        }
    }

    pub fn format(&self, mocked_executables: &[PathBuf]) -> String {
        match self {
            CommandMatcher::ExactMatch(command) => command.format(mocked_executables),
            CommandMatcher::RegexMatch(AnchoredRegex {
                original_string, ..
            }) => original_string.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnchoredRegex {
    original_string: String,
    regex: Regex,
}

impl PartialEq for AnchoredRegex {
    fn eq(&self, other: &AnchoredRegex) -> bool {
        self.original_string == other.original_string
    }
}

impl Eq for AnchoredRegex {}

impl AnchoredRegex {
    pub fn new(raw_regex: &str) -> R<AnchoredRegex> {
        Ok(AnchoredRegex {
            original_string: raw_regex.to_string(),
            regex: Regex::new(&format!("^{}$", raw_regex))?,
        })
    }

    pub fn is_match(&self, other: &str) -> bool {
        self.regex.is_match(other)
    }
}

#[cfg(test)]
mod command_matcher {
    use super::*;

    mod exact_match {
        use super::*;

        #[test]
        fn matches_command_executable() -> R<()> {
            assert!(CommandMatcher::ExactMatch(Command::new("true")?)
                .matches(&[], &Command::new("true")?));
            Ok(())
        }

        #[test]
        fn matches_command_with_arguments() -> R<()> {
            assert!(CommandMatcher::ExactMatch(Command::new("echo 1")?)
                .matches(&[], &Command::new("echo 1")?));
            Ok(())
        }

        #[test]
        fn matches_command_even_if_it_doesnt_exist() -> R<()> {
            assert!(CommandMatcher::ExactMatch(Command::new("foo")?)
                .matches(&[], &Command::new("foo")?));
            Ok(())
        }

        #[test]
        fn matches_command_with_full_path() -> R<()> {
            assert!(CommandMatcher::ExactMatch(Command::new("/bin/true")?)
                .matches(&[], &Command::new("/bin/true")?));
            Ok(())
        }

        #[test]
        fn doesnt_match_a_different_command() -> R<()> {
            assert!(!CommandMatcher::ExactMatch(Command::new("foo")?)
                .matches(&[], &Command::new("bar")?));
            Ok(())
        }

        #[test]
        fn doesnt_match_with_the_same_executable_but_different_arguments() -> R<()> {
            assert!(!CommandMatcher::ExactMatch(Command::new("foo 1")?)
                .matches(&[], &Command::new("foo 2")?));
            Ok(())
        }
    }

    mod regex_match {
        use super::*;

        fn test_regex_matches_command(regex: &str, command: &str) -> R<bool> {
            let result = CommandMatcher::RegexMatch(AnchoredRegex::new(regex)?)
                .matches(&[], &Command::new(command)?);
            Ok(result)
        }

        #[test]
        fn matches_a_command() -> R<()> {
            assert!(test_regex_matches_command("cp .*_bar", "cp foo_bar")?);
            Ok(())
        }

        #[test]
        fn doesnt_match_a_command_if_regex_doesnt_match() -> R<()> {
            assert!(!test_regex_matches_command("foo", "bar")?);
            Ok(())
        }

        #[test]
        fn only_matches_if_entire_command_is_a_match() -> R<()> {
            assert!(!test_regex_matches_command("cp", "cp foo")?);
            Ok(())
        }

        mod anchoring {
            use super::*;

            #[test]
            fn matches_if_both_anchors_are_included() -> R<()> {
                assert!(test_regex_matches_command("^cp$", "cp")?);
                Ok(())
            }

            #[test]
            fn matches_if_front_anchor_is_included() -> R<()> {
                assert!(test_regex_matches_command("^cp", "cp")?);
                Ok(())
            }

            #[test]
            fn matches_if_end_anchor_is_included() -> R<()> {
                assert!(test_regex_matches_command("cp$", "cp")?);
                Ok(())
            }
        }
    }
}
