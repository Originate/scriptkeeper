#![cfg_attr(feature = "ci", deny(warnings))]

use libc::{c_long, c_ulonglong, pid_t};
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::Pid;
use nix::unistd::{execv, fork, getpid, ForkResult};
use std::ffi::{c_void, CString};
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
        println!("offset: {:?}, register: {}", offset, register);
        Ok(register)
    }
}

#[cfg(test)]
mod test_peek {
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

extern "C" {
    fn c_ptrace_peekdata(pid: pid_t, address: c_long) -> c_long;
}

fn ptrace_peekdata(pid: Pid, address: c_ulonglong) -> [u8; 8] {
    unsafe {
        let word = c_ptrace_peekdata(pid.as_raw(), address as c_long);
        let ptr: &[u8; 8] = &*(&word as *const i64 as *const [u8; 8]);
        *ptr
    }
}

fn data_to_string(data: [u8; 8]) -> R<String> {
    let mut result = vec![];
    for char in data.iter() {
        if *char == 0 {
            break;
        }
        result.push(*char);
    }
    Ok(String::from_utf8(result)?)
}

#[cfg(test)]
mod test_data_to_string {
    use super::*;

    #[test]
    fn reads_null_terminated_strings() {
        let data = [102, 111, 111, 0, 0, 0, 0, 0];
        assert_eq!(data_to_string(data).unwrap(), "foo");
    }
}

pub fn first_execve_path(executable: impl ToString) -> R<String> {
    let mut result = None;
    match fork()? {
        ForkResult::Child => {
            ptrace::traceme()?;
            signal::kill(getpid(), Some(Signal::SIGSTOP))?;
            let path = CString::new(executable.to_string())?;
            execv(&path, &[path.clone()])?;
        }
        ForkResult::Parent { child } => {
            waitpid(child, None)?;
            ptrace::setoptions(child, Options::PTRACE_O_TRACESYSGOOD)?;
            ptrace::syscall(child)?;

            loop {
                match waitpid(child, None)? {
                    WaitStatus::Exited(..) => break,
                    WaitStatus::PtraceSyscall(..) => {
                        if result.is_none() {
                            let registers = ptrace::getregs(child)?;
                            if registers.orig_rax == libc::SYS_execve as c_ulonglong
                                && registers.rdi > 0
                            {
                                let word = ptrace_peekdata(child, registers.rdi);
                                result = Some(data_to_string(word)?);
                            }
                        }
                    }
                    _ => {}
                }
                ptrace::syscall(child)?;
            }
        }
    }
    Ok(result.ok_or("execve didn't happen")?)
}

#[cfg(test)]
mod test_first_execve_path {
    use super::*;

    #[test]
    fn returns_the_path_of_the_spawned_executable() {
        assert_eq!(first_execve_path("./foo").unwrap(), "./foo");
    }
}
