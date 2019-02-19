#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]

use check_protocols::{run_main, Context, R};

fn main() -> R<()> {
    run_main(Context::new()?, std::env::args(), &mut std::io::stdout())?;
    Ok(())
}
