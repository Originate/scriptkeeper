use check_protocols::{run_check_protocols, Context, ExitCode, R};
use path::PathBuf;
use std::*;

#[allow(dead_code)]
fn test_run_from_directory(directory: &str) -> R<()> {
    let script_file = PathBuf::from(directory).join("script");
    let output = run_check_protocols(Context::new_test_context(), &script_file)
        .map_err(|error| format!("can't execute {:?}: {}", &script_file, error))?;
    let expected_file = PathBuf::from(directory).join("expected");
    let expected = String::from_utf8(
        fs::read(&expected_file)
            .map_err(|error| format!("error reading {:?}: {}", &expected_file, error))?,
    )?;
    assert_eq!(output.0, ExitCode(0));
    assert_eq!(output.1, expected);
    Ok(())
}
