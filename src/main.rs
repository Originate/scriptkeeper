use libc::c_long;
use nix::sys::ptrace;
use nix::sys::wait::wait;
use nix::unistd::{execv, fork, ForkResult, Pid};
use std::ffi::{c_void, CString};
use std::ptr::null_mut;

type R<A> = Result<A, Box<std::error::Error>>;

fn main() -> R<()> {
    let result = fork()?;
    match result {
        ForkResult::Child => {
            ptrace::traceme()?;
            execv(&CString::new("./foo")?, &vec![CString::new("./foo")?])?;
        }
        ForkResult::Parent { child } => {
            println!("wait result: {:?}", wait());
            rust_peekuser(child)?;
            ptrace::cont(child, None)?;
        }
    }
    Ok(())
}

fn rust_peekuser(child: Pid) -> R<()> {
    let offset = (libc::ORIG_RAX * 8) as *mut c_void;
    #[allow(deprecated)]
    unsafe {
        let register: c_long = libc::ptrace(
            libc::PTRACE_PEEKUSER,
            child,
            offset,
            null_mut() as *mut c_void,
        );
        println!(
            "offset: {:?} - {}, register: {}",
            offset, offset as u64, register
        );
    }
    Ok(())
}
