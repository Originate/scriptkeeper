#![cfg_attr(feature = "ci", deny(warnings))]

use libc::c_long;
use nix::unistd::Pid;
use std::ffi::c_void;
use std::ptr::null_mut;

pub type R<A> = Result<A, Box<std::error::Error>>;

pub fn peek_orig_rax(child: Pid) -> R<c_long> {
    let offset = libc::ORIG_RAX * 8;
    peek_register(child, offset)
}

pub fn peek_register(child: Pid, offset: i32) -> R<c_long> {
    #[allow(deprecated)]
    unsafe {
        let register: c_long = libc::ptrace(
            libc::PTRACE_PEEKUSER,
            child,
            offset as *mut c_void,
            null_mut() as *mut c_void,
        );
        println!(
            "offset: {:?} - {}, register: {}",
            offset, offset as u64, register
        );
        Ok(register)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nix::sys::ptrace;
    use nix::sys::wait::waitpid;
    use nix::unistd::{execv, fork, ForkResult};
    use std::ffi::CString;

    #[test]
    fn tracks_syscalls() {
        let result = fork().unwrap();
        match result {
            ForkResult::Child => {
                ptrace::traceme().unwrap();
                execv(
                    &CString::new("./foo").unwrap(),
                    &vec![CString::new("./foo").unwrap()],
                )
                .unwrap();
            }
            ForkResult::Parent { child } => {
                println!("wait result: {:?}", waitpid(child, None));
                assert_eq!(peek_orig_rax(child).unwrap(), 59);
                ptrace::cont(child, None).unwrap();
            }
        }
    }
}
