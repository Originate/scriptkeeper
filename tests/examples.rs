#[path = "./utils.rs"]
mod utils;

use check_protocols::{run_check_protocols, Context, ExitCode, R};
use path::PathBuf;
use std::*;
use utils::with_cursor;

pub fn test_run_from_directory(directory: &str) -> R<()> {
    let directory = PathBuf::from("./tests/examples").join(directory);
    let script_file = directory.join("script");
    let output = with_cursor(|cursor| {
        run_check_protocols(Context::new_test_context(), &script_file, cursor)
    })
    .map_err(|error| format!("can't execute {:?}: {}", &script_file, error))?;
    let expected_file = directory.join("expected");
    let expected = String::from_utf8(
        fs::read(&expected_file)
            .map_err(|error| format!("error reading {:?}: {}", &expected_file, error))?,
    )?;
    assert_eq!(output.0, ExitCode(0));
    assert_eq!(output.1, expected);
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
