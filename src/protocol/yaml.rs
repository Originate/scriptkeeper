use crate::R;
use linked_hash_map::LinkedHashMap;
use std::fmt;
use std::io;
use std::io::Cursor;
use yaml_rust::{Yaml, YamlEmitter};

pub trait YamlExt {
    fn expect_str(&self) -> R<&str>;

    fn expect_array(&self) -> R<&Vec<Yaml>>;

    fn expect_object(&self) -> R<&LinkedHashMap<Yaml, Yaml>>;

    fn expect_integer(&self) -> R<i32>;
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
    fn expect_field(&self, field: &str) -> R<&Yaml>;
}

impl MapExt for LinkedHashMap<Yaml, Yaml> {
    fn expect_field(&self, field: &str) -> R<&Yaml> {
        Ok(self
            .get(&Yaml::String(field.to_string()))
            .ok_or_else(|| format!("expected field '{}', got: {:?}", field, self))?)
    }
}

fn adjust_yaml_output(input: Vec<u8>) -> Vec<u8> {
    let mut result: Vec<u8> = input.into_iter().skip(4).collect();
    result.push(b'\n');
    result
}

pub fn write_yaml(mut output_stream: Box<io::Write>, yaml: &Yaml) -> R<()> {
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
