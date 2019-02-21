extern crate yaml_rust;

mod yaml;

use crate::protocol::yaml::*;
use crate::utils::path_to_string;
use crate::R;
use std::collections::vec_deque::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
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
    pub stdout: Vec<u8>,
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
                stdout: vec![],
            })
        }
        match yaml {
            Yaml::String(string) => from_string(string),
            Yaml::Hash(object) => {
                let mut step = from_string(object.expect_field("command")?.expect_str()?)?;
                if let Ok(stdout) = object.expect_field("stdout") {
                    step.stdout = stdout.expect_str()?.bytes().collect();
                }
                Ok(step)
            }
            _ => Err(format!("expected: string or array, got: {:?}", yaml))?,
        }
    }

    pub fn format_error(expected: &str, received: &str) -> String {
        format!("  expected: {}\n  received: {}\n", expected, received)
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
                stdout: vec![],
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
                stdout: vec![],
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
                stdout: vec![],
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
                stdout: vec![],
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
                stdout: b"bar".to_vec(),
            },
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct Protocol {
    pub steps: VecDeque<Step>,
    pub arguments: Vec<String>,
}

impl Protocol {
    #[allow(dead_code)]
    pub fn empty() -> Protocol {
        Protocol {
            steps: VecDeque::new(),
            arguments: vec![],
        }
    }

    fn parse(yaml: Yaml) -> R<Protocol> {
        fn from_array(array: &[Yaml]) -> R<Protocol> {
            Ok(Protocol {
                steps: array
                    .iter()
                    .map(Step::parse)
                    .collect::<R<VecDeque<Step>>>()?,
                arguments: vec![],
            })
        }
        Ok(match yaml {
            Yaml::Array(array) => from_array(&array)?,
            Yaml::Hash(object) => {
                let mut protocol = from_array(object.expect_field("protocol")?.expect_array()?)?;
                if let Ok(arguments) = object.expect_field("arguments") {
                    protocol.arguments = arguments
                        .expect_str()?
                        .split_whitespace()
                        .map(String::from)
                        .collect();
                }
                protocol
            }
            _ => Err(format!("expected: array or object, got: {:?}", yaml))?,
        })
    }

    pub fn load(executable_path: &Path) -> R<Vec<Protocol>> {
        let file_contents = read_protocols_file(executable_path)?;
        let yaml: Vec<Yaml> = YamlLoader::load_from_str(&file_contents)?;
        yaml.into_iter().map(Protocol::parse).collect()
    }
}

#[cfg(test)]
mod load {
    use super::*;
    use crate::R;
    use std::path::PathBuf;
    use std::*;
    use test_utils::{trim_margin, Mappable, TempFile};

    fn test_parse(protocol_string: &str) -> R<Protocol> {
        let tempfile = TempFile::new()?;
        let protocols_file = tempfile.path().with_extension("protocols.yaml");
        fs::write(&protocols_file, trim_margin(protocol_string)?)?;
        let result = Protocol::load(&tempfile.path())?;
        assert_eq!(result.len(), 1);
        Ok(result.into_iter().next().unwrap())
    }

    #[test]
    fn reads_a_protocol_from_a_sibling_yaml_file() -> R<()> {
        assert_eq!(
            test_parse(
                r##"
                    |- /bin/true
                "##,
            )?,
            Protocol {
                steps: vec![Step {
                    command: "/bin/true".to_string(),
                    arguments: vec![],
                    stdout: vec![],
                }]
                .into(),
                arguments: vec![]
            },
        );
        Ok(())
    }

    #[test]
    fn returns_an_informative_error_when_the_protocol_file_is_missing() {
        assert_eq!(
            format!(
                "{}",
                Protocol::load(&PathBuf::from("./does-not-exist")).unwrap_err()
            ),
            "protocol file not found: ./does-not-exist.protocols.yaml"
        );
    }

    #[test]
    fn works_for_multiple_commands() -> R<()> {
        assert_eq!(
            test_parse(
                r##"
                    |- /bin/true
                    |- /bin/false
                "##
            )?
            .steps
            .map(|step| step.command),
            vec!["/bin/true", "/bin/false"],
        );
        Ok(())
    }

    #[test]
    fn allows_to_specify_arguments() -> R<()> {
        assert_eq!(
            test_parse(
                r##"
                    |- /bin/true foo bar
                "##
            )?
            .steps
            .map(|step| step.arguments),
            vec![vec!["foo", "bar"].map(String::from)],
        );
        Ok(())
    }

    #[test]
    fn allows_to_specify_the_protocol_as_an_object() -> R<()> {
        assert_eq!(
            test_parse(
                r##"
                    |protocol:
                    |  - /bin/true
                "##
            )?,
            Protocol {
                steps: vec![Step {
                    command: "/bin/true".to_string(),
                    arguments: vec![],
                    stdout: vec![],
                }]
                .into(),
                arguments: vec![]
            },
        );
        Ok(())
    }

    #[test]
    fn allows_to_specify_script_arguments() -> R<()> {
        assert_eq!(
            test_parse(
                r##"
                    |protocol:
                    |  - /bin/true
                    |arguments: "foo bar"
                "##
            )?
            .arguments,
            vec!["foo", "bar"]
        );
        Ok(())
    }
}

fn add_extension(path: &Path, extension: &str) -> PathBuf {
    let mut path = path.to_path_buf().into_os_string();
    path.push(".");
    path.push(extension);
    PathBuf::from(path)
}

fn read_protocols_file(executable_path: &Path) -> R<String> {
    let protocols_file = add_extension(executable_path, "protocols.yaml");
    if !protocols_file.exists() {
        Err(format!(
            "protocol file not found: {}",
            protocols_file.to_string_lossy()
        ))?;
    }
    Ok(match fs::read(&protocols_file) {
        Err(error) => Err(format!(
            "error reading {}: {}",
            path_to_string(&protocols_file)?,
            error
        ))?,
        Ok(file_contents) => String::from_utf8(file_contents)?,
    })
}

#[cfg(test)]
mod read_protocols_file {
    use super::*;
    use std::fs;
    use test_utils::TempFile;

    #[test]
    fn reads_sibling_protocol_files() -> R<()> {
        let tempfile = TempFile::new()?;
        let sibling_file = format!("{}.protocols.yaml", path_to_string(&tempfile.path())?);
        fs::write(sibling_file, "foo")?;
        assert_eq!(read_protocols_file(&tempfile.path())?, "foo");
        Ok(())
    }

    #[test]
    fn works_for_files_with_extensions() -> R<()> {
        let tempfile = TempFile::new()?;
        let file = tempfile.path().with_extension("ext");
        let sibling_file = format!("{}.protocols.yaml", path_to_string(&file)?);
        fs::write(sibling_file, "foo")?;
        assert_eq!(read_protocols_file(&file)?, "foo");
        Ok(())
    }
}
