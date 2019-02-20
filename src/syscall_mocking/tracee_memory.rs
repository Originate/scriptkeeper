use crate::R;
use libc::{c_ulonglong, c_void};
use nix::sys::ptrace;
use nix::unistd::Pid;

fn cast_to_byte_array(word: c_ulonglong) -> [u8; 8] {
    let ptr: &[u8; 8];
    unsafe {
        ptr = &*(&word as *const u64 as *const [u8; 8]);
    }
    *ptr
}

fn peekdata(pid: Pid, address: c_ulonglong) -> R<c_ulonglong> {
    Ok(ptrace::read(pid, address as *mut c_void)? as c_ulonglong)
}

pub fn peekdata_iter(pid: Pid, address: c_ulonglong) -> impl Iterator<Item = R<c_ulonglong>> {
    struct Iter {
        pid: Pid,
        address: c_ulonglong,
    };

    impl Iterator for Iter {
        type Item = R<c_ulonglong>;

        fn next(&mut self) -> Option<Self::Item> {
            let result = peekdata(self.pid, self.address);
            self.address += 8;
            Some(result)
        }
    }

    Iter { pid, address }
}

pub fn data_to_string(data: impl Iterator<Item = R<c_ulonglong>>) -> R<String> {
    let mut result = vec![];
    'outer: for word in data {
        for char in cast_to_byte_array(word?).iter() {
            if *char == 0 {
                break 'outer;
            }
            result.push(*char);
        }
    }
    Ok(String::from_utf8(result)?)
}

pub fn peek_string_array(pid: Pid, address: c_ulonglong) -> R<Vec<String>> {
    let mut result = vec![];
    for word in peekdata_iter(pid, address).skip(1) {
        let word = word?;
        if word == 0 {
            break;
        }
        let arg = data_to_string(peekdata_iter(pid, word as c_ulonglong))?;
        result.push(arg);
    }
    Ok(result)
}

fn cast_to_word(bytes: [u8; 8]) -> c_ulonglong {
    let void_ptr;
    unsafe {
        void_ptr = std::mem::transmute(bytes);
    }
    void_ptr
}

pub fn pokedata(pid: Pid, address: c_ulonglong, words: c_ulonglong) -> R<()> {
    ptrace::write(pid, address as *mut c_void, words as *mut c_void)?;
    Ok(())
}

pub fn string_to_data(string: &str) -> R<c_ulonglong> {
    if string.len() >= 8 {
        Err("string_to_data: string too long")?
    } else {
        let mut result = [0, 0, 0, 0, 0, 0, 0, 0];
        for (i, char) in string.as_bytes().iter().enumerate() {
            result[i] = *char;
        }
        Ok(cast_to_word(result))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod peeking {
        use super::*;

        #[test]
        fn reads_null_terminated_strings_from_one_word() {
            let data = vec![[102, 111, 111, 0, 0, 0, 0, 0]]
                .into_iter()
                .map(cast_to_word)
                .map(Ok);
            assert_eq!(data_to_string(data).unwrap(), "foo");
        }

        #[test]
        fn works_for_multiple_words() {
            let data = vec![
                [97, 98, 99, 100, 101, 102, 103, 104],
                [105, 0, 0, 0, 0, 0, 0, 0],
            ]
            .into_iter()
            .map(cast_to_word)
            .map(Ok);
            assert_eq!(data_to_string(data).unwrap(), "abcdefghi");
        }

        #[test]
        fn works_when_null_is_on_the_edge() {
            let data = vec![
                [97, 98, 99, 100, 101, 102, 103, 104],
                [0, 0, 0, 0, 0, 0, 0, 0],
            ]
            .into_iter()
            .map(cast_to_word)
            .map(Ok);
            assert_eq!(data_to_string(data).unwrap(), "abcdefgh");
        }
    }

    mod poking {
        use super::*;
        use crate::syscall_mocking::fork_with_child_errors;
        use nix::sys::ptrace::Options;
        use nix::sys::signal;
        use nix::sys::signal::Signal;
        use nix::sys::wait::{waitpid, WaitStatus};
        use nix::unistd::{execv, getpid};
        use std::ffi::CString;

        mod string_to_data {
            use super::*;

            #[test]
            fn converts_strings_to_bytes() -> R<()> {
                assert_eq!(
                    string_to_data("foo")?,
                    cast_to_word([102, 111, 111, 0, 0, 0, 0, 0])
                );
                Ok(())
            }

            #[test]
            fn errors_on_too_long_strings() -> R<()> {
                assert_eq!(
                    format!("{}", string_to_data("1234567890").unwrap_err()),
                    "string_to_data: string too long"
                );
                assert_eq!(
                    format!("{}", string_to_data("12345678").unwrap_err()),
                    "string_to_data: string too long"
                );
                assert_eq!(
                    string_to_data("1234567")?,
                    cast_to_word([49, 50, 51, 52, 53, 54, 55, 0])
                );
                Ok(())
            }
        }

        #[test]
        fn roundtrip() -> R<()> {
            let fork_result = fork_with_child_errors(
                || {
                    ptrace::traceme()?;
                    signal::kill(getpid(), Some(Signal::SIGSTOP))?;
                    let path = CString::new("/bin/true")?;
                    execv(&path, &[path.clone()])?;
                    Ok(())
                },
                |child| -> R<()> {
                    waitpid(child, None)?;
                    ptrace::setoptions(child, Options::PTRACE_O_TRACESYSGOOD)?;
                    ptrace::syscall(child)?;

                    loop {
                        let status = waitpid(child, None)?;
                        match status {
                            WaitStatus::Exited(..) => {
                                break;
                            }
                            WaitStatus::PtraceSyscall(..) => {
                                let registers = ptrace::getregs(child)?;
                                if registers.orig_rax == libc::SYS_execve as c_ulonglong {
                                    pokedata(child, registers.rdi, string_to_data("/foo")?)?;
                                    let result =
                                        data_to_string(peekdata_iter(child, registers.rdi))?;
                                    assert_eq!(result, "/foo");
                                }
                            }
                            _ => {}
                        }
                        ptrace::syscall(child)?;
                    }
                    Ok(())
                },
            );
            assert_eq!(
                format!("{}", fork_result.unwrap_err()),
                "ENOENT: No such file or directory",
                "unexpected error"
            );
            Ok(())
        }
    }
}
