use crate::R;
use linked_hash_map::LinkedHashMap;
use yaml_rust::Yaml;

pub trait YamlExt {
    fn expect_str(&self) -> R<&str>;

    fn expect_array(&self) -> R<&Vec<Yaml>>;

    fn expect_object(&self) -> R<&LinkedHashMap<Yaml, Yaml>>;
}

impl YamlExt for Yaml {
    fn expect_str(&self) -> R<&str> {
        Ok(self
            .as_str()
            .ok_or_else(|| format!("expected: string, got: {:?}", self))?)
    }

    fn expect_array(&self) -> R<&Vec<Yaml>> {
        Ok(self
            .as_vec()
            .ok_or_else(|| format!("expected: array, got: {:?}", self))?)
    }

    fn expect_object(&self) -> R<&LinkedHashMap<Yaml, Yaml>> {
        Ok(self
            .as_hash()
            .ok_or_else(|| format!("expected: object, got: {:?}", self))?)
    }
}

pub trait MapExt {
    fn expect_field(&self, field: &str) -> R<&Yaml>;
}

impl MapExt for LinkedHashMap<Yaml, Yaml> {
    fn expect_field(&self, field: &str) -> R<&Yaml> {
        Ok(self
            .get(&Yaml::String(field.to_string()))
            .ok_or_else(|| format!("expected field '{}', got: {:?}", field, self))?)
    }
}

pub fn split_words(string: String) -> R<Vec<String>> {
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
                Some(char) => self.parse_error(&format!("unknown escaped character {}", char))?,
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

    Iter {
        original: string.clone(),
        input: string.chars().collect(),
        acc: "".to_string(),
    }
    .parse_words()
}

#[cfg(test)]
mod split_words {
    use super::*;

    #[test]
    fn splits_words() -> R<()> {
        assert_eq!(split_words("foo bar".to_string())?, vec!["foo", "bar"]);
        Ok(())
    }

    #[test]
    fn honors_quotes() -> R<()> {
        assert_eq!(
            split_words(r#"foo "bar baz""#.to_string())?,
            vec!["foo", "bar baz"]
        );
        Ok(())
    }

    #[test]
    fn honors_escaped_quotes_outside_quotes() -> R<()> {
        assert_eq!(
            split_words(r#"foo\" bar baz"#.to_string())?,
            vec!["foo\"", "bar", "baz"]
        );
        assert_eq!(
            split_words(r#"foo\" "bar baz""#.to_string())?,
            vec!["foo\"", "bar baz"]
        );
        Ok(())
    }

    #[test]
    fn honors_escaped_quotes_inside_quotes() -> R<()> {
        assert_eq!(
            split_words(r#"foo "bar\" baz""#.to_string())?,
            vec!["foo", "bar\" baz"]
        );
        Ok(())
    }

    #[test]
    fn quotes_next_to_letters() -> R<()> {
        assert_eq!(
            format!("{}", split_words(r#"foo"bar""#.to_string()).unwrap_err()),
            r#"opening quotes must be preceeded by a space ("foo\"bar\"")"#
        );
        assert_eq!(
            format!("{}", split_words(r#""foo"bar"#.to_string()).unwrap_err()),
            r#"closing quotes must be followed by a space ("\"foo\"bar")"#
        );
        Ok(())
    }

    #[test]
    fn nonmatching_quotes() -> R<()> {
        assert_eq!(
            format!("{}", split_words(r#"foo "bar"#.to_string()).unwrap_err()),
            r#"unmatched quotes ("foo \"bar")"#
        );
        Ok(())
    }

    #[test]
    fn double_spaces() -> R<()> {
        assert_eq!(split_words("foo  bar".to_string())?, vec!["foo", "bar"]);
        Ok(())
    }

    #[test]
    fn leading_spaces() -> R<()> {
        assert_eq!(split_words(" foo bar".to_string())?, vec!["foo", "bar"]);
        Ok(())
    }

    #[test]
    fn trailing_spaces() -> R<()> {
        assert_eq!(split_words("foo bar ".to_string())?, vec!["foo", "bar"]);
        Ok(())
    }

    mod escaping {
        use super::*;

        #[test]
        fn newlines() -> R<()> {
            assert_eq!(
                split_words(r#"foo "bar\nbaz""#.to_string())?,
                vec!["foo", "bar\nbaz"]
            );
            Ok(())
        }

        #[test]
        fn escaping_spaces() -> R<()> {
            assert_eq!(
                split_words(r#"foo bar\ baz"#.to_string())?,
                vec!["foo", "bar baz"]
            );
            Ok(())
        }

        #[test]
        fn escaping_backslashes() -> R<()> {
            assert_eq!(
                split_words(r#"foo bar\\baz"#.to_string())?,
                vec!["foo", r#"bar\baz"#]
            );
            Ok(())
        }
    }
}
