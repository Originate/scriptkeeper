use libc::c_long;
use nix::unistd::Pid;
use std::ffi::c_void;
use std::ptr::null_mut;

pub type R<A> = Result<A, Box<std::error::Error>>;

pub fn peekuser(child: Pid) -> R<()> {
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
