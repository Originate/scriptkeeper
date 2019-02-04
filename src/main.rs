use nix::sys::ptrace;
use nix::sys::wait::wait;
use nix::unistd::{execv, fork, ForkResult};
use std::ffi::CString;
use tracing_poc::{peekuser, R};

fn main() -> R<()> {
    let result = fork()?;
    match result {
        ForkResult::Child => {
            ptrace::traceme()?;
            execv(&CString::new("./foo")?, &[CString::new("./foo")?])?;
        }
        ForkResult::Parent { child } => {
            println!("wait result: {:?}", wait());
            peekuser(child)?;
            ptrace::cont(child, None)?;
        }
    }
    Ok(())
}
