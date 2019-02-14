#![cfg_attr(feature = "dev", allow(dead_code, unused_variables, unused_imports))]
#![cfg_attr(feature = "ci", deny(warnings))]

mod emulation;
mod protocol;
mod syscall_mocking;
mod tracee_memory;
mod utils;

use crate::emulation::emulate_executable;
use protocol::TestResult;
use std::path::Path;

pub type R<A> = Result<A, Box<std::error::Error>>;

pub fn run(script: &Path) -> R<String> {
    let test_result = TestResult {
        expected: protocol::load(script)?,
        received: emulate_executable(script)?,
    };
    Ok(test_result.format())
}
