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

fn cast_to_word(bytes: [u8; 8]) -> c_ulonglong {
    let void_ptr;
    unsafe {
        void_ptr = std::mem::transmute(bytes);
    }
    void_ptr
}

fn peekdata(pid: Pid, address: c_ulonglong) -> R<c_ulonglong> {
    Ok(ptrace::read(pid, address as *mut c_void)? as c_ulonglong)
}

fn peekdata_iter(pid: Pid, address: c_ulonglong) -> impl Iterator<Item = R<c_ulonglong>> {
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

fn data_to_string(data: impl Iterator<Item = R<c_ulonglong>>) -> R<Vec<u8>> {
    let mut result = vec![];
    'outer: for word in data {
        for char in cast_to_byte_array(word?).iter() {
            if *char == 0 {
                break 'outer;
            }
            result.push(*char);
        }
    }
    Ok(result)
}

pub fn peek_string(pid: Pid, address: c_ulonglong) -> R<Vec<u8>> {
    data_to_string(peekdata_iter(pid, address))
}

pub fn peek_string_array(pid: Pid, address: c_ulonglong) -> R<Vec<Vec<u8>>> {
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

#[allow(dead_code)]
pub fn peek_bytes(pid: Pid, address: c_ulonglong, count: usize) -> R<Vec<u8>> {
    let iter = peekdata_iter(pid, address);
    let mut vec = vec![];
    for word in iter {
        vec.append(&mut cast_to_byte_array(word?).to_vec());
        if vec.len() >= count {
            break;
        }
    }
    Ok(vec.into_iter().take(count).collect())
}

#[cfg(test)]
mod peeking {
    use super::*;

    #[test]
    fn reads_null_terminated_strings_from_one_word() {
        let data = vec![[102, 111, 111, 0, 0, 0, 0, 0]]
            .into_iter()
            .map(cast_to_word)
            .map(Ok);
        assert_eq!(data_to_string(data).unwrap(), b"foo");
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
        assert_eq!(data_to_string(data).unwrap(), b"abcdefghi");
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
        assert_eq!(data_to_string(data).unwrap(), b"abcdefgh");
    }
}

fn pokedata(pid: Pid, address: c_ulonglong, words: c_ulonglong) -> R<()> {
    ptrace::write(pid, address as *mut c_void, words as *mut c_void)?;
    Ok(())
}

fn string_to_data(string: &[u8], max_size: c_ulonglong) -> R<Vec<c_ulonglong>> {
    if string.len() as c_ulonglong >= max_size {
        Err("string_to_data: string too long")?
    } else {
        let mut result = vec![];
        let number_of_words = (string.len() / 8) + 1;
        for word_number in 0..number_of_words {
            let mut word = [0, 0, 0, 0, 0, 0, 0, 0];
            for i in 0..8 {
                if let Some(char) = string.get(word_number * 8 + i) {
                    word[i] = *char;
                }
            }
            result.push(cast_to_word(word));
        }
        Ok(result)
    }
}

pub fn poke_string(
    pid: Pid,
    mut address: c_ulonglong,
    string: &[u8],
    max_size: c_ulonglong,
) -> R<()> {
    for word in string_to_data(string, max_size)? {
        pokedata(pid, address, word)?;
        address += 8;
    }
    Ok(())
}

pub fn poke_single_word_string(pid: Pid, address: c_ulonglong, string: &[u8]) -> R<()> {
    poke_string(pid, address, string, 8)
}

#[cfg(test)]
mod string_to_data {
    use super::*;
    use test_utils::assert_error;

    #[test]
    fn converts_strings_to_bytes() -> R<()> {
        assert_eq!(
            string_to_data(b"foo", 8)?,
            vec![cast_to_word([102, 111, 111, 0, 0, 0, 0, 0])]
        );
        Ok(())
    }

    #[test]
    fn works_for_longer_strings() -> R<()> {
        assert_eq!(
            string_to_data(b"foo_foo_foo", 16)?,
            vec![
                cast_to_word([102, 111, 111, 95, 102, 111, 111, 95]),
                cast_to_word([102, 111, 111, 0, 0, 0, 0, 0]),
            ]
        );
        Ok(())
    }

    #[test]
    fn errors_on_too_long_strings() -> R<()> {
        assert_error!(
            string_to_data(b"1234567890", 8),
            "string_to_data: string too long"
        );
        assert_error!(
            string_to_data(b"12345678", 8),
            "string_to_data: string too long"
        );
        assert_eq!(
            string_to_data(b"1234567", 8)?,
            vec![cast_to_word([49, 50, 51, 52, 53, 54, 55, 0])]
        );
        assert_error!(
            string_to_data(b"123456781234567890", 16),
            "string_to_data: string too long"
        );
        assert_error!(
            string_to_data(b"1234567812345678", 16),
            "string_to_data: string too long"
        );
        assert_eq!(
            string_to_data(b"123456781234567", 16)?,
            vec![
                cast_to_word([49, 50, 51, 52, 53, 54, 55, 56]),
                cast_to_word([49, 50, 51, 52, 53, 54, 55, 0])
            ]
        );
        Ok(())
    }

    #[test]
    fn adds_another_word_for_null_termination_if_necessary() -> R<()> {
        assert_eq!(
            string_to_data(b"12345678", 16)?,
            vec![
                cast_to_word([49, 50, 51, 52, 53, 54, 55, 56]),
                cast_to_word([0, 0, 0, 0, 0, 0, 0, 0])
            ]
        );
        Ok(())
    }
}

#[cfg(test)]
mod roundtrip {
    use super::*;
    use crate::syscall_mocking::fork_with_child_errors;
    use libc::user_regs_struct;
    use nix::sys::ptrace::Options;
    use nix::sys::signal;
    use nix::sys::signal::Signal;
    use nix::sys::wait::{waitpid, WaitStatus};
    use nix::unistd::{execv, getpid};
    use std::env;
    use std::ffi::CString;
    use test_utils::assert_error;

    fn run_roundtrip_test(test: fn(child: Pid, registers: user_regs_struct) -> R<()>) -> R<()> {
        fork_with_child_errors(
            || {
                ptrace::traceme()?;
                signal::kill(getpid(), Some(Signal::SIGSTOP))?;
                env::current_dir()?;
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
                            if registers.orig_rax == libc::SYS_getcwd as c_ulonglong {
                                test(child, registers)?;
                            }
                        }
                        _ => {}
                    }
                    ptrace::syscall(child)?;
                }
                Ok(())
            },
        )
    }

    #[test]
    fn run_roundtrip_test_runs_the_given_test() {
        assert_error!(run_roundtrip_test(|_, _| Err("foo")?), "foo");
    }

    #[test]
    fn short_string() -> R<()> {
        run_roundtrip_test(|child, registers| {
            poke_single_word_string(child, registers.rdi, b"foo")?;
            assert_eq!(peek_string(child, registers.rdi)?, b"foo");
            Ok(())
        })
    }

    #[test]
    fn long_string() -> R<()> {
        run_roundtrip_test(|child, registers| {
            poke_string(child, registers.rdi, b"foo_bar_baz", 16)?;
            assert_eq!(peek_string(child, registers.rdi)?, b"foo_bar_baz");
            Ok(())
        })
    }

    mod peek_bytes {
        use super::*;

        #[test]
        fn short_count() -> R<()> {
            run_roundtrip_test(|child, registers| {
                poke_string(child, registers.rdi, b"foo_bar", 16)?;
                assert_eq!(peek_bytes(child, registers.rdi, 3)?, b"foo");
                Ok(())
            })
        }

        #[test]
        fn longer_count() -> R<()> {
            run_roundtrip_test(|child, registers| {
                poke_string(child, registers.rdi, b"foo_bar_baz", 16)?;
                assert_eq!(peek_bytes(child, registers.rdi, 10)?, b"foo_bar_ba");
                Ok(())
            })
        }
    }
}
