use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempdir::TempDir;
use trim_margin::MarginTrimmable;

type R<A> = Result<A, Box<std::error::Error>>;

pub fn trim_margin(str: &str) -> R<String> {
    Ok(format!(
        "{}\n",
        str.trim_margin().ok_or("include a margin prefix '|'")?
    ))
}

fn run(command: &str, args: Vec<&str>) -> R<()> {
    let status = Command::new(command).args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err("command failed")?
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

    pub fn write_temp_script(script: &str) -> R<TempFile> {
        let tempfile = TempFile::new()?;
        fs::write(&tempfile.path(), script.trim_start())?;
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
