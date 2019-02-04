use libc::{c_long, user_regs_struct};
use nix::sys::ptrace;
use nix::sys::ptrace::Request;
// use nix::sys::wait::wait;
use nix::unistd::{fork, ForkResult, Pid};
use std::ffi::{c_void, CString};
use std::mem::size_of;
use std::ptr::{null, null_mut};

fn main() -> Result<(), Box<std::error::Error>> {
    let result = fork()?;
    match result {
        ForkResult::Child => {
            #[allow(deprecated)]
            unsafe {
                println!(
                    "traceme: {:?}",
                    ptrace::ptrace(
                        Request::PTRACE_TRACEME,
                        Pid::from_raw(0),
                        null_mut(),
                        null_mut()
                    )?
                );
            }
            let path = CString::new("/bin/ls")?;
            let path_ptr: *const i8 = path.as_ptr();
            let arg0 = CString::new("ls")?;
            let end_marker: *const i8 = null();
            unsafe {
                libc::execl(path_ptr, arg0.as_ptr(), end_marker);
            }
        }
        ForkResult::Parent { child } => {
            println!("parent... ({})", child);
            println!("size: {:?}", size_of::<user_regs_struct>() / 8);
            unsafe {
                println!("wait result: {:?}", libc::wait(null_mut()));
            }
            let i = 15;
            let offset = (i * 8) as *mut c_void;
            unsafe {
                #[allow(deprecated)]
                let register: c_long =
                    ptrace::ptrace(Request::PTRACE_PEEKUSER, child, offset, null_mut())?;
                println!(
                    "offset: {:?} - {}, register: {}",
                    offset, offset as u64, register
                );
                #[allow(deprecated)]
                ptrace::ptrace(Request::PTRACE_CONT, child, null_mut(), null_mut())?;
            };
        }
    }
    Ok(())
}
