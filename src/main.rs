#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]

use path::PathBuf;
use std::*;
use tracing_poc::{run, R};

fn main() -> R<()> {
    let mut args = env::args();
    args.next();
    print!(
        "{}",
        run(&PathBuf::from(args.next().ok_or("supply one argument")?))?
    );
    Ok(())
}
