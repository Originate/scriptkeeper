#![cfg_attr(feature = "ci", deny(warnings))]

use tracing_poc::{first_execve_path, R};

fn main() -> R<()> {
    println!("executable: {}", first_execve_path("./foo")?);
    Ok(())
}
