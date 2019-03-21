use super::command::Command;
use super::executable_path;
use crate::R;
use regex::Regex;
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
    pub fn matches(&self, received: &Command) -> bool {
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
        assert!(CommandMatcher::exact_match("cp ./")?.matches(&Command::new("cp ./")?));
        Ok(())
    }

    #[test]
    fn exact_command_doesnt_match_different_command() -> R<()> {
        assert!(!CommandMatcher::exact_match("cp ./")?.matches(&Command::new("bar")?));
        Ok(())
    }

    fn test_regex_matches_command(regex: &str, command: &str) -> R<bool> {
        let result = CommandMatcher::regex_match(regex)?.matches(&Command::new(command)?);
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
