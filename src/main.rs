#![cfg_attr(feature = "ci", deny(warnings))]

use std::path::Path;
use std::*;
use tracing_poc::{execve_paths, R};

fn main() -> R<()> {
    let mut args = env::args();
    args.next();
    println!(
        "executable: {:?}",
        execve_paths(Path::new(&args.next().ok_or("supply one argument")?))?
    );
    Ok(())
}
