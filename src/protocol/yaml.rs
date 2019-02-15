use crate::R;
use linked_hash_map::LinkedHashMap;
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
