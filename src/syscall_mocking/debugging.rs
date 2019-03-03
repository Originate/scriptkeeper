use super::syscall::Syscall;
use super::syscall::Syscall::*;
use super::tracee_memory;
use super::SyscallStop;
use crate::R;
use libc::{c_ulonglong, user_regs_struct};
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
            Close => vec![("fd", registers.rdi.to_string())],
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
            Openat => vec![("filename", Debugger::string(pid, registers.rsi))],
            _ => vec![],
        }
    }

    pub fn log_syscall(
        &mut self,
        pid: Pid,
        syscall_stop: &SyscallStop,
        syscall: &Syscall,
        registers: &user_regs_struct,
    ) -> R<()> {
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
                        let return_value = registers.rax;
                        eprintln!("{} -> {}", message, return_value);
                        enter_log_messages.remove(&pid);
                    }
                },
            },
        }
        Ok(())
    }
}
