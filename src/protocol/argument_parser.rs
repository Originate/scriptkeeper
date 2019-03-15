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

    fn parse_char(&mut self, excluded: &[char], is_regex: bool) -> R<Option<char>> {
        Ok(match self.input.pop_front() {
            None => None,
            Some('\\') if is_regex && self.input.get(0) == Some(&'`') => Some('`'),
            Some('\\') if !is_regex => Some(match self.input.pop_front() {
                None => self.parse_error("a backslash must be followed by a character")?,
                Some('"') => '"',
                Some('n') => '\n',
                Some(' ') => ' ',
                Some('\\') => '\\',
                // Some('$') => '$',
                // Some('`') => '`',
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

    fn collect_chars_until(&mut self, excluded: &[char], is_regex: bool) -> R<String> {
        let mut result = "".to_string();
        while let Some(char) = self.parse_char(excluded, is_regex)? {
            result.push(char);
        }
        Ok(result)
    }

    fn parse_balanced_chars(
        &mut self,
        character: char,
        is_regex: bool,
        unclosed_error: &str,
    ) -> R<Option<String>> {
        self.skip_char(character, "shouldn't happen")?;
        let argument = self.collect_chars_until(&[character], is_regex)?;
        self.skip_char(character, unclosed_error)?;
        self.peek_chars(
            vec![Some(' '), None],
            &format!("closing '{:?}' must be followed by a space", character),
        )?;
        Ok(Some(argument))
    }

    fn parse_word(&mut self) -> R<Option<Argument>> {
        self.skip_spaces();
        Ok(match self.input.get(0) {
            None => None,
            Some('$') if self.input.get(1) == Some(&'`') => {
                self.skip_char('$', "shouldn't happen")?;
                self.parse_balanced_chars('`', true, "unclosed regular expression")?
                    .map(|regexp| Argument::Regex(regexp))
            }
            Some('"') => self
                .parse_balanced_chars('"', false, "unmatched quotes")?
                .map(|word| Argument::Word(word)),
            Some(_) => {
                let result = self.collect_chars_until(&[' ', '"'], false)?;
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
