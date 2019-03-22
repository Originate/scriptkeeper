use super::command::Command;
use super::executable_path;
use crate::R;
use regex::Regex;
use std::str;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CommandMatcher {
    ExactMatch(Command),
    RegexMatch(AnchoredRegex),
}

impl CommandMatcher {
    pub fn matches(&self, received: &Command) -> bool {
        match self {
            CommandMatcher::ExactMatch(command) => {
                executable_path::compare_executables(&command.executable, &received.executable)
                    && command.arguments == received.arguments
            }
            CommandMatcher::RegexMatch(regex) => regex.is_match(&received.format()),
        }
    }

    pub fn format(&self) -> String {
        match self {
            CommandMatcher::ExactMatch(command) => command.format(),
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

    #[test]
    fn exact_command_matches_received() -> R<()> {
        assert!(CommandMatcher::ExactMatch(Command::new("cp ./")?).matches(&Command::new("cp ./")?));
        Ok(())
    }

    #[test]
    fn exact_command_doesnt_match_different_command() -> R<()> {
        assert!(!CommandMatcher::ExactMatch(Command::new("cp ./")?).matches(&Command::new("bar")?));
        Ok(())
    }

    fn test_regex_matches_command(regex: &str, command: &str) -> R<bool> {
        let result =
            CommandMatcher::RegexMatch(AnchoredRegex::new(regex)?).matches(&Command::new(command)?);
        Ok(result)
    }

    #[test]
    fn regex_matches_received() -> R<()> {
        assert!(test_regex_matches_command("cp \\w", "cp a")?);
        Ok(())
    }

    #[test]
    fn regex_doesnt_match_received_if_regex_doesnt_match() -> R<()> {
        assert!(!test_regex_matches_command("cp \\d", "cp a")?);
        Ok(())
    }

    #[test]
    fn regex_only_matches_if_entire_command_is_a_match() -> R<()> {
        assert!(!test_regex_matches_command("cp \\d", "cp 1 2")?);
        Ok(())
    }

    #[test]
    fn regex_still_matches_if_both_anchors_are_included() -> R<()> {
        assert!(test_regex_matches_command("^cp \\d$", "cp 1")?);
        Ok(())
    }

    #[test]
    fn regex_still_matches_if_front_anchor_is_included() -> R<()> {
        assert!(test_regex_matches_command("^cp \\d", "cp 1")?);
        Ok(())
    }

    #[test]
    fn regex_still_matches_if_end_anchor_is_included() -> R<()> {
        assert!(test_regex_matches_command("cp \\d$", "cp 1")?);
        Ok(())
    }
}
