use super::syscall::Syscall;
use super::syscall::Syscall::*;
use super::tracee_memory;
use super::SyscallStop;
use crate::R;
use libc::{c_longlong, c_ulonglong, user_regs_struct};
use nix::errno;
use nix::sys::ptrace;
use nix::unistd::Pid;
use std::collections::HashMap;
use std::env;

pub enum Debugger {
    Disabled,
    Debugger {
        enter_log_messages: HashMap<Pid, String>,
    },
}

impl Debugger {
    pub fn new() -> Debugger {
        match env::var_os("DEBUG") {
            None => Debugger::Disabled,
            Some(_) => Debugger::Debugger {
                enter_log_messages: HashMap::new(),
            },
        }
    }

    fn string(pid: Pid, address: c_ulonglong) -> String {
        let string = match tracee_memory::peek_string(pid, address) {
            Err(error) => error.to_string(),
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        };
        format!("{:?}", string)
    }

    fn string_array(pid: Pid, address: c_ulonglong) -> String {
        if address == 0 {
            "NULL".to_string()
        } else {
            match tracee_memory::peek_string_array(pid, address) {
                Err(error) => error.to_string(),
                Ok(bytes_array) => {
                    let mut result = vec![];
                    for bytes in bytes_array {
                        result.push(String::from_utf8_lossy(&bytes).into_owned());
                    }
                    format!("{:?}", result)
                }
            }
        }
    }

    fn syscall_arguments(
        pid: Pid,
        syscall: &Syscall,
        registers: &user_regs_struct,
    ) -> Vec<(&'static str, String)> {
        match syscall {
            Read => vec![("fd", registers.rdi.to_string())],
            Write => vec![
                ("fd", registers.rdi.to_string()),
                ("buf", Debugger::string(pid, registers.rsi)),
                ("count", registers.rdx.to_string()),
            ],
            Open => vec![("filename", Debugger::string(pid, registers.rdi))],
            Close => vec![("fd", registers.rdi.to_string())],
            Stat => vec![("filename", Debugger::string(pid, registers.rdi))],
            Fstat => vec![("fd", registers.rdi.to_string())],
            Dup2 => vec![
                ("oldfd", registers.rdi.to_string()),
                ("newfd", registers.rsi.to_string()),
            ],
            Execve => vec![
                ("filename", Debugger::string(pid, registers.rdi)),
                ("argv", Debugger::string_array(pid, registers.rsi)),
                ("envp", Debugger::string_array(pid, registers.rdx)),
            ],
            Fcntl => vec![
                ("fd", registers.rdi.to_string()),
                ("cmd", format!("{:?}", FcntlCmd::from(registers.rsi))),
            ],
            Openat => vec![("filename", Debugger::string(pid, registers.rsi))],
            _ => vec![],
        }
    }

    fn format_return_value(registers: &user_regs_struct) -> String {
        let return_value = registers.rax;
        if (return_value as c_longlong) < 0 {
            errno::from_i32(-(return_value as i32)).to_string()
        } else {
            return_value.to_string()
        }
    }

    pub fn log_syscall<F: FnOnce() -> R<()>>(
        &mut self,
        pid: Pid,
        syscall_stop: &SyscallStop,
        syscall: &Syscall,
        make_syscall: F,
    ) -> R<()> {
        match syscall_stop {
            SyscallStop::Enter => {
                self.log_syscall_details(pid, syscall_stop, syscall)?;
                make_syscall()?;
            }
            SyscallStop::Exit => {
                make_syscall()?;
                self.log_syscall_details(pid, syscall_stop, syscall)?;
            }
        }
        Ok(())
    }

    fn log_syscall_details(
        &mut self,
        pid: Pid,
        syscall_stop: &SyscallStop,
        syscall: &Syscall,
    ) -> R<()> {
        let registers = ptrace::getregs(pid)?;
        match self {
            Debugger::Disabled => {}
            Debugger::Debugger {
                ref mut enter_log_messages,
            } => match syscall_stop {
                SyscallStop::Enter => {
                    let mut arguments_string = "".to_string();
                    for (name, value) in
                        Debugger::syscall_arguments(pid, &syscall, &registers).into_iter()
                    {
                        if arguments_string != "" {
                            arguments_string.push_str(", ");
                        }
                        arguments_string.push_str(&format!("{}: {}", name, value));
                    }
                    let message = format!("{:?}({})", &syscall, &arguments_string);
                    enter_log_messages.insert(pid, message);
                }
                SyscallStop::Exit => match enter_log_messages.get(&pid) {
                    None => eprintln!("error: exit without enter"),
                    Some(message) => {
                        eprintln!(
                            "{} -> {}",
                            message,
                            Debugger::format_return_value(&registers)
                        );
                        enter_log_messages.remove(&pid);
                    }
                },
            },
        }
        Ok(())
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
enum FcntlCmd {
    F_DUPFD,
    F_GETFD,
    F_SETFD,
    F_GETFL,
    F_SETFL,
    Unknown(c_ulonglong),
}

impl From<c_ulonglong> for FcntlCmd {
    fn from(fcntl_cmd: c_ulonglong) -> Self {
        match fcntl_cmd {
            0 => FcntlCmd::F_DUPFD,
            1 => FcntlCmd::F_GETFD,
            2 => FcntlCmd::F_SETFD,
            3 => FcntlCmd::F_GETFL,
            4 => FcntlCmd::F_SETFL,
            unknown => FcntlCmd::Unknown(unknown),
        }
    }
}
