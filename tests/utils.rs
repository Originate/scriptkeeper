use path::PathBuf;
use std::*;
use tracing_poc::{run, R};

#[allow(dead_code)]
fn run_high_level_test(directory: &str) -> R<()> {
    let output = run(&PathBuf::from(directory).join("script"))?;
    let expected = String::from_utf8(fs::read(PathBuf::from(directory).join("expected"))?)?;
    assert_eq!(output, expected);
    Ok(())
}
