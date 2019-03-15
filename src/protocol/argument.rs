#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Argument {
    Word(String),
    Regex(String),
}

impl Argument {
    pub fn wrap_words(words: Vec<Vec<u8>>) -> Vec<Argument> {
        words
            .into_iter()
            .map(|word| Argument::Word(String::from_utf8_lossy(&word).to_string()))
            .collect()
    }

    pub fn inner_string(&self) -> &String {
        match self {
            Argument::Word(string) => &string,
            Argument::Regex(string) => &string,
        }
    }
}
