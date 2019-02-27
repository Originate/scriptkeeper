#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

use check_protocols::{run_main, wrap_main, Context};

fn main() {
    wrap_main(
        |exitcode| exitcode.exit(),
        || run_main(Context::new()?, std::env::args(), &mut std::io::stdout()),
    );
}
