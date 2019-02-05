use nix::sys::ptrace;
use nix::sys::wait::wait;
use nix::unistd::{execv, fork, ForkResult};
use std::ffi::CString;
use tracing_poc::{peek_register, R};

fn main() -> R<()> {
    let result = fork()?;
    match result {
        ForkResult::Child => {
            ptrace::traceme()?;
            execv(&CString::new("./foo")?, &[CString::new("./foo")?])?;
        }
        ForkResult::Parent { child } => {
            println!("wait result: {:?}", wait());
            for i in 0..27 {
                let offset = 8 * i;
                peek_register(child, offset)?;
            }
            ptrace::cont(child, None)?;
        }
    }
    Ok(())
}
