use crate::R;
use libc::{c_uint, c_ulonglong, c_void};
use nix::sys::ptrace;
use nix::unistd::Pid;

fn cast_to_four_byte_array(x: c_uint) -> [u8; 4] {
    [
        (x & 0xff) as u8,
        ((x >> 8) & 0xff) as u8,
        ((x >> 16) & 0xff) as u8,
        ((x >> 24) & 0xff) as u8,
    ]
}

fn cast_to_eight_byte_array(x: c_ulonglong) -> [u8; 8] {
    [
        (x & 0xff) as u8,
        ((x >> 8) & 0xff) as u8,
        ((x >> 16) & 0xff) as u8,
        ((x >> 24) & 0xff) as u8,
        ((x >> 32) & 0xff) as u8,
        ((x >> 40) & 0xff) as u8,
        ((x >> 48) & 0xff) as u8,
        ((x >> 56) & 0xff) as u8,
    ]
}

fn cast_to_eight_byte_word(bytes: [u8; 8]) -> c_ulonglong {
    u64::from(bytes[0])
        + (u64::from(bytes[1]) << 8)
        + (u64::from(bytes[2]) << 16)
        + (u64::from(bytes[3]) << 24)
        + (u64::from(bytes[4]) << 32)
        + (u64::from(bytes[5]) << 40)
        + (u64::from(bytes[6]) << 48)
        + (u64::from(bytes[7]) << 56)
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
        for char in cast_to_eight_byte_array(word?).iter() {
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

#[cfg(test)]
mod peeking {
    use super::*;

    #[test]
    fn reads_null_terminated_strings_from_one_word() {
        let data = vec![[102, 111, 111, 0, 0, 0, 0, 0]]
            .into_iter()
            .map(cast_to_eight_byte_word)
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
        .map(cast_to_eight_byte_word)
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
        .map(cast_to_eight_byte_word)
        .map(Ok);
        assert_eq!(data_to_string(data).unwrap(), b"abcdefgh");
    }
}

pub fn poke_four_bytes(pid: Pid, address: c_ulonglong, small_word: c_uint) -> R<()> {
    let existing_word: c_ulonglong = peekdata(pid, address)?;
    let mut overlapping_data_array: [u8; 8] = cast_to_eight_byte_array(existing_word);

    let first_four_bytes: [u8; 4] = cast_to_four_byte_array(small_word);
    overlapping_data_array[..4].clone_from_slice(&first_four_bytes);

    let new_word: c_ulonglong = cast_to_eight_byte_word(overlapping_data_array);
    pokedata(pid, address, new_word)
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
            result.push(cast_to_eight_byte_word(word));
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
mod poking {
    use super::*;
    use crate::tracer::fork_with_child_errors;
    use nix::sys::ptrace::Options;
    use nix::sys::signal;
    use nix::sys::signal::Signal;
    use nix::sys::wait::{waitpid, WaitStatus};
    use nix::unistd::{execv, getpid};
    use std::env;
    use std::ffi::CString;
    use test_utils::assert_error;

    mod string_to_data {
        use super::*;

        #[test]
        fn converts_strings_to_bytes() -> R<()> {
            assert_eq!(
                string_to_data(b"foo", 8)?,
                vec![cast_to_eight_byte_word([102, 111, 111, 0, 0, 0, 0, 0])]
            );
            Ok(())
        }

        #[test]
        fn works_for_longer_strings() -> R<()> {
            assert_eq!(
                string_to_data(b"foo_foo_foo", 16)?,
                vec![
                    cast_to_eight_byte_word([102, 111, 111, 95, 102, 111, 111, 95]),
                    cast_to_eight_byte_word([102, 111, 111, 0, 0, 0, 0, 0]),
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
                vec![cast_to_eight_byte_word([49, 50, 51, 52, 53, 54, 55, 0])]
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
                    cast_to_eight_byte_word([49, 50, 51, 52, 53, 54, 55, 56]),
                    cast_to_eight_byte_word([49, 50, 51, 52, 53, 54, 55, 0])
                ]
            );
            Ok(())
        }

        #[test]
        fn adds_another_word_for_null_termination_if_necessary() -> R<()> {
            assert_eq!(
                string_to_data(b"12345678", 16)?,
                vec![
                    cast_to_eight_byte_word([49, 50, 51, 52, 53, 54, 55, 56]),
                    cast_to_eight_byte_word([0, 0, 0, 0, 0, 0, 0, 0])
                ]
            );
            Ok(())
        }
    }

    mod roundtrip {
        use super::*;
        use libc::user_regs_struct;

        fn cast_to_four_byte_word(bytes: [u8; 4]) -> c_uint {
            u32::from(bytes[0])
                + (u32::from(bytes[1]) << 8)
                + (u32::from(bytes[2]) << 16)
                + (u32::from(bytes[3]) << 24)
        }

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
                                    assert!(registers.rsi >= 512);
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
        fn roundtrip_for_four_byte_word_doesnt_clobber_adjacent_four_bytes() -> R<()> {
            run_roundtrip_test(|child, registers| {
                let eight_bytes: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
                let four_bytes: [u8; 4] = [10, 20, 30, 40];

                pokedata(child, registers.rdi, cast_to_eight_byte_word(eight_bytes))?;
                poke_four_bytes(child, registers.rdi, cast_to_four_byte_word(four_bytes))?;

                assert_eq!(
                    cast_to_eight_byte_array(peekdata(child, registers.rdi)?),
                    [10, 20, 30, 40, 5, 6, 7, 8],
                );
                Ok(())
            })
        }

        #[test]
        fn roundtrip_for_a_single_word() -> R<()> {
            run_roundtrip_test(|child, registers| {
                poke_single_word_string(child, registers.rdi, b"foo")?;
                assert_eq!(peek_string(child, registers.rdi)?, b"foo");
                Ok(())
            })
        }

        #[test]
        fn roundtrip_for_multiple_words() -> R<()> {
            run_roundtrip_test(|child, registers| {
                poke_string(child, registers.rdi, b"foo_bar_baz", 16)?;
                assert_eq!(peek_string(child, registers.rdi)?, b"foo_bar_baz");
                Ok(())
            })
        }
    }
}
