extern crate yaml_rust;

mod yaml;

use crate::protocol::yaml::YamlExt;
use crate::utils::path_to_string;
use crate::R;
use std::fs;
use std::path::Path;
use yaml_rust::{Yaml, YamlLoader};

#[derive(PartialEq, Eq, Debug)]
pub struct Step {
    pub command: String,
    pub arguments: Vec<String>,
}

impl Step {
    fn format(&self) -> String {
        let mut words = vec![self.command.clone()];
        words.append(&mut self.arguments.clone());
        words.join(" ")
    }
}

pub type Protocol = Vec<Step>;

fn read_protocol_file(executable_path: &Path) -> R<String> {
    let protocol_file = executable_path.with_extension("protocol.yaml");
    if !protocol_file.exists() {
        Err(format!(
            "protocol file not found: {}",
            path_to_string(&protocol_file)?
        ))?;
    }
    Ok(match fs::read(&protocol_file) {
        Err(error) => Err(format!(
            "error reading {}: {}",
            path_to_string(&protocol_file)?,
            error
        ))?,
        Ok(file_contents) => String::from_utf8(file_contents)?,
    })
}

fn parse_yaml(yaml: Yaml) -> R<Protocol> {
    let mut result = vec![];
    for step in yaml.expect_array()? {
        let step: &str = step.expect_str()?;
        let mut words = step.split_whitespace();
        let (command, arguments) = {
            match words.next() {
                None => Err(format!("expected: command and arguments, got: {:?}", step))?,
                Some(command) => (command.to_string(), words.map(String::from).collect()),
            }
        };
        result.push(Step { command, arguments });
    }
    Ok(result)
}

pub fn load(executable_path: &Path) -> R<Protocol> {
    let file_contents = read_protocol_file(executable_path)?;
    let yaml: Vec<Yaml> = YamlLoader::load_from_str(&file_contents)?;
    let document: Yaml = {
        if yaml.len() != 1 {
            Err(format!("expected: single yaml document, got: {:?}", yaml))?;
        }
        yaml.into_iter().next().unwrap()
    };
    parse_yaml(document)
}

#[cfg(test)]
mod load {

    use super::*;
    use crate::R;
    use map_in_place::MapVecInPlace;
    use std::path::PathBuf;
    use std::*;
    use test_utils::{trim_margin, TempFile};

    fn test_read_protocol(protocol_string: &str, expected: Vec<(&str, Vec<&str>)>) -> R<()> {
        let tempfile = TempFile::new()?;
        let protocol_file = tempfile.path().with_extension("protocol.yaml");
        fs::write(&protocol_file, trim_margin(protocol_string)?)?;
        assert_eq!(
            load(&tempfile.path())?,
            expected.map(|(command, args)| Step {
                command: command.to_string(),
                arguments: args.map(String::from)
            })
        );
        Ok(())
    }

    #[test]
    fn reads_a_protocol_from_a_sibling_yaml_file() -> R<()> {
        test_read_protocol(
            r##"
              |- /bin/true
            "##,
            vec![("/bin/true", vec![])],
        )
    }

    #[test]
    fn works_for_multiple_commands() -> R<()> {
        test_read_protocol(
            r##"
              |- /bin/true
              |- /bin/false
            "##,
            vec![("/bin/true", vec![]), ("/bin/false", vec![])],
        )
    }

    #[test]
    fn allows_to_specify_arguments() -> R<()> {
        test_read_protocol(
            r##"
              |- /bin/true foo bar
            "##,
            vec![("/bin/true", vec!["foo", "bar"])],
        )
    }

    #[test]
    fn returns_an_informative_error_when_the_protocol_file_is_missing() {
        assert_eq!(
            format!("{}", load(&PathBuf::from("./does-not-exist")).unwrap_err()),
            "protocol file not found: ./does-not-exist.protocol.yaml"
        );
    }
}

pub struct TestResult {
    pub expected: Protocol,
    pub received: Protocol,
}

impl TestResult {
    pub fn format(&self) -> String {
        let mut expected_iter = self.expected.iter();
        let mut received_iter = self.received.iter();
        loop {
            match (expected_iter.next(), received_iter.next()) {
                (Some(expected_step), Some(received_step)) => {
                    if received_step != expected_step {
                        return TestResult::format_error(
                            &expected_step.format(),
                            &received_step.format(),
                        );
                    }
                }
                (Some(expected_step), None) => {
                    return TestResult::format_error(&expected_step.format(), "<script terminated>");
                }
                (None, Some(received_step)) => {
                    return TestResult::format_error("<protocol end>", &received_step.format());
                }
                (None, None) => {
                    break;
                }
            }
        }
        "All tests passed.\n".to_string()
    }

    fn format_error(expected: &str, received: &str) -> String {
        format!("error:\nexpected: {}\nreceived: {}\n", expected, received)
    }
}
