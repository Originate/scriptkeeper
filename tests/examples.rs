#![cfg_attr(
    feature = "dev",
    allow(dead_code, unused_variables, unused_imports, unreachable_code)
)]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

#[path = "./utils.rs"]
mod utils;

use path::PathBuf;
use scriptkeeper::{context::Context, run_scriptkeeper, ExitCode, R};
use std::*;

pub fn test_run_from_directory(directory: &str) -> R<()> {
    let directory = PathBuf::from("./tests/examples").join(directory);
    let script_file = directory.join("script");
    let context = &Context::new_mock();
    let exitcode = run_scriptkeeper(context, &script_file)
        .map_err(|error| format!("can't execute {:?}: {}", &script_file, error))?;
    let expected_file = directory.join("expected");
    let expected = String::from_utf8(
        fs::read(&expected_file)
            .map_err(|error| format!("error reading {:?}: {}", &expected_file, error))?,
    )?;
    print!("{}", context.get_captured_stdout());
    eprintln!("{}", context.get_captured_stderr());
    assert_eq!(exitcode, ExitCode(0));
    assert_eq!(context.get_captured_stdout(), expected);
    Ok(())
}

macro_rules! example {
    ($directory:ident) => {
        #[test]
        fn $directory() -> R<()> {
            test_run_from_directory(stringify!($directory))?;
            Ok(())
        }
    };
}

example!(simple);
example!(bigger);
