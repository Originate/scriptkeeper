extern crate yaml_rust;

mod yaml;

use crate::protocol::yaml::*;
use crate::utils::path_to_string;
use crate::R;
use std::collections::vec_deque::VecDeque;
use std::fs;
use std::path::Path;
use yaml_rust::{Yaml, YamlLoader};

pub fn format_command(command: &str, mut arguments: Vec<String>) -> String {
    let mut words = vec![command.to_string()];
    words.append(&mut arguments);
    words.join(" ")
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Step {
    pub command: String,
    pub arguments: Vec<String>,
    pub stdout: String,
}

impl Step {
    fn parse(yaml: &Yaml) -> R<Step> {
        fn from_string(string: &str) -> R<Step> {
            let mut words = string.split_whitespace();
            let (command, arguments) = {
                match words.next() {
                    None => Err(format!(
                        "expected: space-separated command and arguments, got: {:?}",
                        string
                    ))?,
                    Some(command) => (command.to_string(), words.map(String::from).collect()),
                }
            };
            Ok(Step {
                command,
                arguments,
                stdout: "".to_string(),
            })
        }
        match yaml {
            Yaml::String(string) => from_string(string),
            Yaml::Hash(object) => {
                let mut step = from_string(object.expect_field("command")?.expect_str()?)?;
                if let Some(stdout) = object.get(&Yaml::String("stdout".to_string())) {
                    step.stdout = stdout.expect_str()?.to_string();
                }
                Ok(step)
            }
            _ => Err(format!("expected: string or array, got: {:?}", yaml))?,
        }
    }

    pub fn format_error(expected: &str, received: &str) -> String {
        format!("error:\nexpected: {}\nreceived: {}\n", expected, received)
    }

    pub fn compare(&self, command: &str, arguments: Vec<String>) -> Result<(), String> {
        if self.command != command || self.arguments != arguments {
            Err(Step::format_error(
                &format_command(&self.command, self.arguments.clone()),
                &format_command(command, arguments),
            ))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod parse_step {
    use super::*;
    use yaml_rust::Yaml;

    fn test_parse_step(yaml: &str, expected: &Step) -> R<()> {
        let yaml = YamlLoader::load_from_str(yaml)?;
        assert_eq!(yaml.len(), 1);
        let yaml = &yaml[0];
        assert_eq!(&Step::parse(yaml)?, expected);
        Ok(())
    }

    #[test]
    fn parses_strings_to_steps() -> R<()> {
        test_parse_step(
            r#""foo""#,
            &Step {
                command: "foo".to_string(),
                arguments: vec![],
                stdout: "".to_string(),
            },
        )?;
        Ok(())
    }

    #[test]
    fn parses_arguments() -> R<()> {
        test_parse_step(
            r#""foo bar""#,
            &Step {
                command: "foo".to_string(),
                arguments: vec!["bar".to_string()],
                stdout: "".to_string(),
            },
        )?;
        Ok(())
    }

    #[test]
    fn parses_objects_to_steps() -> R<()> {
        test_parse_step(
            r#"{command: "foo"}"#,
            &Step {
                command: "foo".to_string(),
                arguments: vec![],
                stdout: "".to_string(),
            },
        )?;
        Ok(())
    }

    #[test]
    fn allows_to_put_arguments_in_the_command_field() -> R<()> {
        test_parse_step(
            r#"{command: "foo bar"}"#,
            &Step {
                command: "foo".to_string(),
                arguments: vec!["bar".to_string()],
                stdout: "".to_string(),
            },
        )?;
        Ok(())
    }

    #[test]
    fn gives_nice_parse_errors() {
        assert_eq!(
            format!("{}", Step::parse(&Yaml::Null).unwrap_err()),
            "expected: string or array, got: Null"
        )
    }

    #[test]
    fn allows_to_specify_stdout() -> R<()> {
        test_parse_step(
            r#"{command: "foo", stdout: "bar"}"#,
            &Step {
                command: "foo".to_string(),
                arguments: vec![],
                stdout: "bar".to_string(),
            },
        )?;
        Ok(())
    }
}

pub type Protocol = VecDeque<Step>;

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

fn parse_protocol(yaml: Yaml) -> R<Protocol> {
    yaml.expect_array()?
        .iter()
        .map(Step::parse)
        .collect::<R<Protocol>>()
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
    parse_protocol(document)
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
                arguments: args.map(String::from),
                stdout: "".to_string()
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
