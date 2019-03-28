use crate::R;
use linked_hash_map::LinkedHashMap;
use std::fmt;
use std::io;
use std::io::Cursor;
use yaml_rust::{yaml::HashNode, Node, Yaml, YamlEmitter, YamlMarked, YamlNode};

pub trait YamlExt {
    type Child;

    fn expect_str(&self) -> R<&str>;

    fn expect_array(&self) -> R<&Vec<Self::Child>>;

    fn expect_object(&self) -> R<&LinkedHashMap<Self::Child, Self::Child>>;

    fn expect_integer(&self) -> R<i32>;
}

impl<T> YamlExt for T
where
    T: fmt::Debug + YamlNode,
    <T as YamlNode>::Child: fmt::Debug,
{
    type Child = <T as YamlNode>::Child;

    fn expect_str(&self) -> R<&str> {
        Ok(self
            .as_str()
            .ok_or_else(|| format!("expected: string, got: {:?}", self))?)
    }

    fn expect_array(&self) -> R<&Vec<Self::Child>> {
        Ok(self
            .as_vec()
            .ok_or_else(|| format!("expected: array, got: {:?}", self))?)
    }

    fn expect_object(&self) -> R<&LinkedHashMap<Self::Child, Self::Child>> {
        Ok(self
            .as_hash()
            .ok_or_else(|| format!("expected: object, got: {:?}", self))?)
    }

    fn expect_integer(&self) -> R<i32> {
        let result: i64 = self
            .as_i64()
            .ok_or_else(|| format!("expected: integer, got: {:?}", self))?;
        if result > i64::from(i32::max_value()) {
            Err(format!(
                "expected: integer below {}, got: {:?}",
                i32::max_value(),
                self
            ))?;
        }
        Ok(result as i32)
    }
}

#[cfg(test)]
mod yaml_ext {
    use super::*;
    use test_utils::assert_error;

    mod expect_integer {
        use super::*;

        #[test]
        fn errors_on_out_of_bounds_integers() -> R<()> {
            let too_large: i64 = (i64::from(i32::max_value())) + 1;
            let yaml = Yaml::Integer(too_large);
            assert_error!(
                yaml.expect_integer(),
                format!(
                    "expected: integer below {}, got: {:?}",
                    i32::max_value(),
                    yaml
                )
            );
            Ok(())
        }
    }
}

pub trait MapExt {
    fn expect_field(&self, field: &str) -> R<&Node>;
}

impl MapExt for LinkedHashMap<Node, Node> {
    fn expect_field(&self, field: &str) -> R<&Node> {
        Ok(self
            .get(&Node(YamlMarked::String(field.to_string()), None))
            .ok_or_else(|| format!("expected field '{}', got: {:?}", field, self))?)
    }
}

pub fn check_keys(known_keys: &[&str], object: &HashNode) -> R<()> {
    for key in object.keys() {
        let key = key.expect_str()?;
        if !known_keys.contains(&key) {
            Err(format!(
                "unexpected field '{}', possible values: {}",
                key,
                known_keys
                    .iter()
                    .map(|key| format!("'{}'", key))
                    .collect::<Vec<String>>()
                    .join(", ")
            ))?;
        }
    }
    Ok(())
}

fn adjust_yaml_output(input: Vec<u8>) -> Vec<u8> {
    let mut result: Vec<u8> = input.into_iter().skip(4).collect();
    result.push(b'\n');
    result
}

pub fn write_yaml(output_stream: &mut dyn io::Write, yaml: &Yaml) -> R<()> {
    struct ToFmtWrite {
        inner: Cursor<Vec<u8>>,
    }

    impl fmt::Write for ToFmtWrite {
        fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
            io::Write::write_all(&mut self.inner, s.as_bytes()).map_err(|_| fmt::Error)
        }
    }
    let mut buffer = ToFmtWrite {
        inner: Cursor::new(vec![]),
    };
    YamlEmitter::new(&mut buffer).dump(yaml)?;
    output_stream.write_all(&adjust_yaml_output(buffer.inner.into_inner()))?;
    Ok(())
}
