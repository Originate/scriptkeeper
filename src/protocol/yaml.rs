use crate::R;
use yaml_rust::Yaml;

pub trait YamlExt {
    fn expect_str(&self) -> R<&str>;

    fn expect_array(&self) -> R<&Vec<Yaml>>;
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
}
