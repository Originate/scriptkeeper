extern crate yaml_rust;

pub mod command;
mod yaml;

use crate::protocol::yaml::*;
use crate::utils::path_to_string;
use crate::R;
pub use command::Command;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use yaml_rust::{yaml::Hash, Yaml, YamlLoader};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Step {
    pub command: Command,
    pub stdout: Vec<u8>,
    pub exitcode: i32,
}

impl Step {
    fn from_string(string: &str) -> R<Step> {
        Ok(Step {
            command: Command::new(string)?,
            stdout: vec![],
            exitcode: 0,
        })
    }

    fn add_exitcode(&mut self, object: &Hash) -> R<()> {
        if let Ok(exitcode) = object.expect_field("exitcode") {
            self.exitcode = exitcode.expect_integer()?;
        }
        Ok(())
    }

    fn add_stdout(&mut self, object: &Hash) -> R<()> {
        if let Ok(stdout) = object.expect_field("stdout") {
            self.stdout = stdout.expect_str()?.bytes().collect();
        }
        Ok(())
    }

    fn parse(yaml: &Yaml) -> R<Step> {
        match yaml {
            Yaml::String(string) => Step::from_string(string),
            Yaml::Hash(object) => {
                let mut step = Step::from_string(object.expect_field("command")?.expect_str()?)?;
                step.add_stdout(object)?;
                step.add_exitcode(object)?;
                Ok(step)
            }
            _ => Err(format!("expected: string or array, got: {:?}", yaml))?,
        }
    }
}

#[cfg(test)]
mod parse_step {
    use super::*;
    use test_utils::assert_error;
    use yaml_rust::Yaml;

    fn test_parse_step(yaml: &str) -> R<Step> {
        let yaml = YamlLoader::load_from_str(yaml)?;
        assert_eq!(yaml.len(), 1);
        let yaml = &yaml[0];
        Step::parse(yaml)
    }

    #[test]
    fn parses_strings_to_steps() -> R<()> {
        assert_eq!(
            test_parse_step(r#""foo""#)?,
            Step {
                command: Command {
                    executable: "foo".to_string(),
                    arguments: vec![],
                },
                stdout: vec![],
                exitcode: 0,
            },
        );
        Ok(())
    }

    #[test]
    fn parses_arguments() -> R<()> {
        assert_eq!(
            test_parse_step(r#""foo bar""#)?.command,
            Command {
                executable: "foo".to_string(),
                arguments: vec!["bar".to_string()],
            },
        );
        Ok(())
    }

    #[test]
    fn parses_objects_to_steps() -> R<()> {
        assert_eq!(
            test_parse_step(r#"{command: "foo"}"#)?,
            Step {
                command: Command {
                    executable: "foo".to_string(),
                    arguments: vec![],
                },
                stdout: vec![],
                exitcode: 0,
            },
        );
        Ok(())
    }

    #[test]
    fn allows_to_put_arguments_in_the_command_field() -> R<()> {
        assert_eq!(
            test_parse_step(r#"{command: "foo bar"}"#)?.command,
            Command {
                executable: "foo".to_string(),
                arguments: vec!["bar".to_string()],
            },
        );
        Ok(())
    }

    #[test]
    fn gives_nice_parse_errors() {
        assert_error!(
            Step::parse(&Yaml::Null),
            "expected: string or array, got: Null"
        )
    }

    #[test]
    fn allows_to_specify_stdout() -> R<()> {
        assert_eq!(
            test_parse_step(r#"{command: "foo", stdout: "bar"}"#)?.stdout,
            b"bar".to_vec(),
        );
        Ok(())
    }

    mod exitcode {
        use super::*;

        #[test]
        fn allows_to_specify_the_mocked_exit_code() -> R<()> {
            assert_eq!(
                test_parse_step(r#"{command: "foo", exitcode: 42}"#)?.exitcode,
                42
            );
            Ok(())
        }

        #[test]
        fn uses_zero_as_the_default() -> R<()> {
            assert_eq!(test_parse_step(r#"{command: "foo"}"#)?.exitcode, 0);
            Ok(())
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Protocol {
    pub steps: VecDeque<Step>,
    pub arguments: Vec<String>,
    pub env: HashMap<String, String>,
}

impl Protocol {
    #[allow(dead_code)]
    pub fn empty() -> Protocol {
        Protocol::new(vec![])
    }

    pub fn new(steps: Vec<Step>) -> Protocol {
        Protocol {
            steps: steps.into(),
            arguments: vec![],
            env: HashMap::new(),
        }
    }

    fn from_array(array: &[Yaml]) -> R<Protocol> {
        Ok(Protocol::new(
            array.iter().map(Step::parse).collect::<R<Vec<Step>>>()?,
        ))
    }

    fn add_arguments(&mut self, object: &Hash) -> R<()> {
        if let Ok(arguments) = object.expect_field("arguments") {
            self.arguments = arguments
                .expect_str()?
                .split_whitespace()
                .map(String::from)
                .collect();
        }
        Ok(())
    }

    fn add_env(&mut self, object: &Hash) -> R<()> {
        if let Ok(env) = object.expect_field("env") {
            for (key, value) in env.expect_object()?.into_iter() {
                self.env.insert(
                    key.expect_str()?.to_string(),
                    value.expect_str()?.to_string(),
                );
            }
        }
        Ok(())
    }

    fn from_object(object: &Hash) -> R<Protocol> {
        let mut protocol = Protocol::from_array(object.expect_field("protocol")?.expect_array()?)?;
        protocol.add_arguments(&object)?;
        protocol.add_env(&object)?;
        Ok(protocol)
    }

    fn parse(yaml: Yaml) -> R<Protocol> {
        Ok(match yaml {
            Yaml::Array(array) => Protocol::from_array(&array)?,
            Yaml::Hash(object) => Protocol::from_object(&object)?,
            _ => Err(format!("expected: array or object, got: {:?}", yaml))?,
        })
    }

    pub fn load(executable_path: &Path) -> R<Vec<Protocol>> {
        let protocols_file = find_protocol_file(executable_path);
        let file_contents = read_protocols_file(&protocols_file)?;
        let yaml: Vec<Yaml> = YamlLoader::load_from_str(&file_contents).map_err(|error| {
            format!(
                "invalid YAML in {}: {}",
                protocols_file.to_string_lossy(),
                error
            )
        })?;
        let result = yaml
            .into_iter()
            .map(Protocol::parse)
            .collect::<R<Vec<Protocol>>>()
            .map_err(|error| {
                format!(
                    "unexpected type in {}: {}",
                    protocols_file.to_string_lossy(),
                    error
                )
            })?;;
        Ok(result)
    }
}

#[cfg(test)]
mod load {
    use super::*;
    use crate::R;
    use std::path::PathBuf;
    use std::*;
    use test_utils::{assert_error, trim_margin, Mappable, TempFile};

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
            Protocol::new(vec![Step {
                command: Command {
                    executable: "/bin/true".to_string(),
                    arguments: vec![],
                },
                stdout: vec![],
                exitcode: 0,
            }]),
        );
        Ok(())
    }

    #[test]
    fn returns_an_informative_error_when_the_protocol_file_is_missing() {
        assert_error!(
            Protocol::load(&PathBuf::from("./does-not-exist")),
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
            .map(|step| step.command.executable),
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
            .map(|step| step.command.arguments),
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
            Protocol::new(vec![Step {
                command: Command {
                    executable: "/bin/true".to_string(),
                    arguments: vec![],
                },
                stdout: vec![],
                exitcode: 0,
            }]),
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

    #[test]
    fn allows_to_specify_the_script_environment() -> R<()> {
        assert_eq!(
            test_parse(
                r##"
                    |protocol:
                    |  - /bin/true
                    |env:
                    |  foo: bar
                "##
            )?
            .env
            .into_iter()
            .collect::<Vec<_>>(),
            vec![("foo".to_string(), "bar".to_string())]
        );
        Ok(())
    }
}

fn find_protocol_file(executable: &Path) -> PathBuf {
    let mut result = executable.to_path_buf().into_os_string();
    result.push(".");
    result.push("protocols.yaml");
    PathBuf::from(result)
}

fn read_protocols_file(protocols_file: &Path) -> R<String> {
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
mod find_protocol_file {
    use super::*;

    #[test]
    fn adds_the_protocols_file_extension() {
        assert_eq!(
            find_protocol_file(&PathBuf::from("foo")),
            PathBuf::from("foo.protocols.yaml")
        );
    }

    #[test]
    fn works_for_files_with_extensions() {
        assert_eq!(
            find_protocol_file(&PathBuf::from("foo.ext")),
            PathBuf::from("foo.ext.protocols.yaml")
        );
    }
}
