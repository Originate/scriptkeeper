#![cfg_attr(feature = "ci", deny(warnings))]

use std::path::Path;
use std::*;
use tracing_poc::{first_execve_path, R};

fn main() -> R<()> {
    let mut args = env::args();
    args.next();
    println!(
        "executable: {}",
        first_execve_path(Path::new(&args.next().ok_or("supply one argument")?))?
    );
    Ok(())
}
