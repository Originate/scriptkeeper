#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]

mod emulation;
mod syscall_mocking;
mod tracee_memory;

use crate::emulation::emulate_executable;
use std::path::Path;

pub type R<A> = Result<A, Box<std::error::Error>>;

pub fn run(script: &Path) -> R<String> {
    Ok(format!("executables: {:?}\n", emulate_executable(script)?))
}
