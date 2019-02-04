use libc::c_long;
use nix::unistd::Pid;
use std::ffi::c_void;
use std::ptr::null_mut;

pub type R<A> = Result<A, Box<std::error::Error>>;

pub fn peekuser(child: Pid) -> R<c_long> {
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
        Ok(register)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nix::sys::ptrace;
    use nix::sys::wait::wait;
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
                println!("wait result: {:?}", wait());
                assert_eq!(peekuser(child).unwrap(), 59);
                ptrace::cont(child, None).unwrap();
            }
        }
    }
}
