pub mod short_temp_files;

use crate::R;
use std::fs;
use std::iter::Peekable;
use std::path::Path;

pub fn path_to_string(path: &Path) -> R<&str> {
    Ok(path
        .to_str()
        .ok_or_else(|| format!("invalid utf8 sequence: {:?}", &path))?)
}

pub fn parse_hashbang(program: &Path) -> Option<String> {
    let contents = fs::read(program).ok()?;
    if contents.starts_with(b"#!") {
        let bytes = contents
            .into_iter()
            .take_while(|&byte| byte != b'\n')
            .collect::<Vec<_>>();
        Some(String::from_utf8_lossy(&bytes).to_string())
    } else {
        None
    }
}

pub fn with_has_more<Element>(
    into_iter: impl IntoIterator<Item = Element>,
) -> impl Iterator<Item = (Element, bool)> {
    struct Iter<Element, I: Iterator<Item = Element>>(Peekable<I>);

    impl<Element, I: Iterator<Item = Element>> Iterator for Iter<Element, I> {
        type Item = (Element, bool);

        fn next(&mut self) -> Option<Self::Item> {
            let result = self.0.next();
            match result {
                None => None,
                Some(current) => Some((current, self.0.peek().is_some())),
            }
        }
    }

    Iter(into_iter.into_iter().peekable())
}
