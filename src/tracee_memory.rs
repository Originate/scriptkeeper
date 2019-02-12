use crate::R;
use libc::{c_ulonglong, c_void};
use nix::sys::ptrace;
use nix::unistd::Pid;

fn ptrace_peekdata(pid: Pid, address: c_ulonglong) -> R<[u8; 8]> {
    unsafe {
        let word = ptrace::read(pid, address as *mut c_void)?;
        let ptr: &[u8; 8] = &*(&word as *const i64 as *const [u8; 8]);
        Ok(*ptr)
    }
}

pub fn ptrace_peekdata_iter(pid: Pid, address: c_ulonglong) -> impl Iterator<Item = R<[u8; 8]>> {
    struct Iter {
        pid: Pid,
        address: c_ulonglong,
    };

    impl Iterator for Iter {
        type Item = R<[u8; 8]>;

        fn next(&mut self) -> Option<Self::Item> {
            let result = ptrace_peekdata(self.pid, self.address);
            self.address += 8;
            Some(result)
        }
    }

    Iter { pid, address }
}

pub fn data_to_string(data: impl Iterator<Item = R<[u8; 8]>>) -> R<String> {
    let mut result = vec![];
    'outer: for word in data {
        for char in word?.iter() {
            if *char == 0 {
                break 'outer;
            }
            result.push(*char);
        }
    }
    Ok(String::from_utf8(result)?)
}

#[cfg(test)]
mod test_data_to_string {
    use super::*;

    #[test]
    fn reads_null_terminated_strings_from_one_word() {
        let data = vec![[102, 111, 111, 0, 0, 0, 0, 0]].into_iter().map(Ok);
        assert_eq!(data_to_string(data).unwrap(), "foo");
    }

    #[test]
    fn works_for_multiple_words() {
        let data = vec![
            [97, 98, 99, 100, 101, 102, 103, 104],
            [105, 0, 0, 0, 0, 0, 0, 0],
        ]
        .into_iter()
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
        .map(Ok);
        assert_eq!(data_to_string(data).unwrap(), "abcdefgh");
    }
}
