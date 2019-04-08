#![deny(clippy::all)]

use std::collections::VecDeque;
use std::env;
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

pub fn with_env<Action, Output>(key: &str, value: &str, action: Action) -> Output
where
    Action: FnOnce() -> Output + std::panic::UnwindSafe,
{
    let outer: Vec<(String, String)> = env::vars().collect();
    env::set_var(key, value);
    let output = std::panic::catch_unwind(|| action());
    env::remove_var(key);
    for (key, value) in outer {
        env::set_var(key, value);
    }
    match output {
        Ok(output) => output,
        Err(error) => panic!(error),
    }
}

#[cfg(test)]
mod with_env {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn executes_the_given_action() {
        let executed = AtomicBool::new(false);
        with_env("FOO", "bar", || {
            executed.store(true, Ordering::SeqCst);
        });
        assert!(executed.into_inner());
    }

    #[test]
    fn allows_to_add_environment_variables() {
        assert_eq!(
            with_env("FOO", "bar", || env::var("FOO")),
            Ok("bar".to_string())
        );
    }

    #[test]
    fn removes_previously_unset_environment_variables() {
        env::remove_var("FOO");
        with_env("FOO", "inner", || {});
        assert_eq!(env::var("FOO"), Err(env::VarError::NotPresent));
    }

    #[test]
    fn restores_the_previous_environment_variable_value() {
        env::set_var("FOO", "outer");
        with_env("FOO", "inner", || {});
        assert_eq!(env::var("FOO"), Ok("outer".to_string()));
    }

    #[test]
    fn restores_the_previous_environment_variable_value_in_case_of_panics() -> R<()> {
        env::set_var("FOO", "outer");
        let _ = std::panic::catch_unwind(|| {
            with_env("FOO", "inner", || panic!());
        });
        assert_eq!(env::var("FOO"), Ok("outer".to_string()));
        Ok(())
    }
}
