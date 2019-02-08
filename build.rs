extern crate trim_margin;

use std::error;
use std::fs;
use std::path::PathBuf;
use trim_margin::MarginTrimmable;

fn main() -> Result<(), Box<error::Error>> {
    let mut code = vec![r##"include!("utils.rs");"##.to_string()];
    let directories = fs::read_dir("./tests")?;
    for directory in directories {
        let directory = directory?;
        if directory.file_type()?.is_dir() {
            code.push(
                format!(
                    r##"
                        |#[test]
                        |fn high_level_{}() -> R<()> {{
                        |    run_high_level_test("{}")?;
                        |    Ok(())
                        |}}
                    "##,
                    directory
                        .path()
                        .file_name()
                        .ok_or("no parent")?
                        .to_str()
                        .ok_or("utf8 error")?,
                    directory.path().to_str().ok_or("utf8 error")?
                )
                .trim_margin()
                .ok_or("include a margin prefix '|'")?,
            );
        }
    }
    fs::write(
        PathBuf::from("tests/high-level-tests-generated.rs"),
        format!("{}\n", code.join("\n")),
    )?;
    Ok(())
}
