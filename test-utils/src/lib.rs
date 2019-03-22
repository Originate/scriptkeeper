#![deny(clippy::all)]

use pretty_assertions::assert_eq;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempdir::TempDir;
use trim_margin::MarginTrimmable;
use yaml_rust::YamlLoader;

type R<A> = Result<A, Box<std::error::Error>>;

pub fn trim_margin(str: &str) -> R<String> {
    Ok(format!(
        "{}\n",
        str.trim_margin().ok_or("include a margin prefix '|'")?
    ))
}

pub fn run(command: &str, args: Vec<&str>) -> R<()> {
    let status = Command::new(&command).args(&args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed: {} {:?}", command, args))?
    }
}

pub struct TempFile {
    tempdir: TempDir,
}

impl TempFile {
    pub fn new() -> R<TempFile> {
        let tempdir = TempDir::new("test")?;
        Ok(TempFile { tempdir })
    }

    pub fn write_temp_script(script: &[u8]) -> R<TempFile> {
        let tempfile = TempFile::new()?;
        fs::write(&tempfile.path(), script)?;
        run("chmod", vec!["+x", tempfile.path().to_str().unwrap()])?;
        Ok(tempfile)
    }

    pub fn path(&self) -> PathBuf {
        self.tempdir.path().join("file")
    }
}

pub trait Mappable<A, B> {
    type Output;

    fn map(self, f: fn(A) -> B) -> Self::Output;
}

impl<A, B> Mappable<A, B> for Vec<A> {
    type Output = Vec<B>;

    fn map(self, f: fn(A) -> B) -> Self::Output {
        self.into_iter().map(f).collect()
    }
}

impl<A, B> Mappable<A, B> for VecDeque<A> {
    type Output = VecDeque<B>;

    fn map(self, f: fn(A) -> B) -> Self::Output {
        self.into_iter().map(f).collect()
    }
}

#[macro_export]
macro_rules! assert_error {
    ($result:expr, $expected:expr) => {
        assert_eq!(format!("{}", $result.unwrap_err()), $expected);
    };
}

pub fn assert_eq_yaml(result: &str, expected: &str) -> R<()> {
    let result =
        YamlLoader::load_from_str(result).map_err(|error| format!("{}\n({})", error, result))?;
    let expected = YamlLoader::load_from_str(expected)
        .map_err(|error| format!("{}\n({})", error, expected))?;
    assert_eq!(result, expected);
    Ok(())
}
