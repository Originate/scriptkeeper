use super::argument::Argument;
use crate::R;
use std::collections::VecDeque;

#[derive(Debug)]
pub(super) struct Parser {
    pub original: String,
    pub input: VecDeque<char>,
}

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
                Some(char) => self.parse_error(&format!("unknown escaped character {}", char))?,
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

    fn parse_word(&mut self) -> R<Option<Argument>> {
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
                Some(Argument::Word(word))
            }
            Some(_) => {
                let result = self.collect_chars_until(&[' ', '"'])?;
                self.peek_chars(
                    vec![Some(' '), None],
                    "opening quotes must be preceeded by a space",
                )?;
                Some(Argument::Word(result))
            }
        })
    }

    pub fn parse_argument_strings(arguments: &str) -> R<Vec<String>> {
        Ok(Parser::parse_arguments(arguments)?
            .into_iter()
            .map(|argument| argument.inner_string().clone())
            .collect())
    }

    pub fn parse_arguments(arguments: &str) -> R<Vec<Argument>> {
        Self {
            original: arguments.to_owned(),
            input: arguments.chars().collect(),
        }
        .collect()
    }
}

impl Iterator for Parser {
    type Item = R<Argument>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.parse_word() {
            Ok(Some(word)) => Some(Ok(word)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
}
