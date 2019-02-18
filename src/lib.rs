#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]

mod emulation;
mod protocol;
mod short_temp_files;
mod syscall_mocking;
mod tracee_memory;
mod utils;

use crate::emulation::run_against_protocol;
use std::path::Path;

pub type R<A> = Result<A, Box<std::error::Error>>;

pub fn run(script: &Path) -> R<String> {
    let expected = protocol::load(script)?;
    let errors = run_against_protocol(script, expected)?;
    Ok(match errors {
        None => "All tests passed.\n".to_string(),
        Some(error) => error,
    })
}
