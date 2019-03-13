#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]
#![deny(clippy::all)]

use check_protocols::{cli::parse_args, context::Context, run_main, wrap_main};

fn main() {
    wrap_main(
        |exitcode| exitcode.exit(),
        || run_main(&Context::new()?, &parse_args(std::env::args())),
    );
}
