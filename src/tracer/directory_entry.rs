use super::tracee_memory;
use crate::R;

// pub const DT_UNKNOWN: u8 = 0;
// pub const DT_FIFO: u8 = 1;
// pub const DT_CHR: u8 = 2;
// pub const DT_DIR: u8 = 4;
// pub const DT_BLK: u8 = 6;
// pub const DT_REG: u8 = 8;
// pub const DT_LNK: u8 = 10;
// pub const DT_SOCK: u8 = 12;

#[derive(Debug, PartialEq, Eq)]
pub enum DirectoryEntry {
    Old(OldDirent),
    New(NewDirent),
}

impl DirectoryEntry {
    pub fn new(bytes: Vec<u8>) -> R<DirectoryEntry> {
        let length = bytes.len();
        let dirent = match bytes.get(18) {
            Some(byte) if *byte > 12 => DirectoryEntry::Old(OldDirent::new(bytes)?),
            Some(byte) if *byte <= 12 => DirectoryEntry::New(NewDirent::new(bytes)?),
            _ if length < 24 => Err(format!(
                "the smallest valid 'dirent' struct is 24 bytes, only {:?} bytes given",
                length
            ))?,
            _ => Err(format!(
                "could not find a valid 'dirent' struct in: {:?}",
                bytes
            ))?,
        };
        Ok(dirent)
    }

    pub fn new_collection_from_buffer(buffer: &Vec<u8>) -> R<Vec<DirectoryEntry>> {
        let mut output: Vec<DirectoryEntry> = vec![];
        let mut start_ptr = 0;
        let record_length_offset = 16;

        loop {
            if let Some(record_length) = buffer.get(start_ptr + record_length_offset) {
                let endpoint = start_ptr + *record_length as usize;
                let dirent = OldDirent::new(buffer[start_ptr..endpoint].iter().cloned().collect())?;
                output.push(DirectoryEntry::Old(dirent));
                start_ptr = endpoint;
            } else {
                break;
            }
        }

        Ok(output)
    }

    pub fn size(&self) -> u16 {
        match self {
            DirectoryEntry::Old(dirent) => dirent.d_reclen,
            DirectoryEntry::New(dirent) => dirent.d_reclen,
        }
    }

    pub fn name(&self) -> String {
        match self {
            DirectoryEntry::Old(dirent) => String::from_utf8_lossy(&dirent.d_name).into_owned(),
            DirectoryEntry::New(dirent) => String::from_utf8_lossy(&dirent.d_name).into_owned(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            DirectoryEntry::Old(dirent) => dirent.to_bytes(),
            DirectoryEntry::New(dirent) => dirent.to_bytes(),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct NewDirent {
    d_ino: u64,
    d_off: u64,
    d_reclen: u16,
    d_type: u8,
    d_name: Vec<u8>,
}

impl NewDirent {
    pub fn new(bytes: Vec<u8>) -> R<NewDirent> {
        let length = bytes.len();
        let (d_ino, d_off, d_reclen) = bytes_to_similar_pieces(&bytes);
        assert!(length == d_reclen as usize);
        let d_type = bytes[18];
        let d_name = get_name_from_bytes(&bytes, 19);

        Ok(NewDirent {
            d_ino,
            d_off,
            d_reclen,
            d_type,
            d_name,
        })
    }

    pub fn new_with_name(name: Vec<u8>) -> NewDirent {
        let d_reclen = (((name.len() as f32 - 5.0) / 8.0).ceil() as u16) * 8 + 24;
        NewDirent {
            d_ino: 0,
            d_off: 0,
            d_reclen,
            d_type: libc::DT_REG,
            d_name: name,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let padding_size: usize = self.d_reclen as usize - 19 - self.d_name.len();
        let string_padding: Vec<u8> = vec![0; padding_size];
        tracee_memory::cast_to_eight_byte_array(self.d_ino)
            .iter()
            .chain(tracee_memory::cast_to_eight_byte_array(self.d_off).iter())
            .chain(tracee_memory::cast_to_two_byte_array(self.d_reclen).iter())
            .chain([self.d_type].iter())
            .chain(self.d_name.iter())
            .chain(string_padding.iter())
            .cloned()
            .collect()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct OldDirent {
    d_ino: u64,
    d_off: u64,
    d_reclen: u16,
    d_name: Vec<u8>,
    pad: u8,
    d_type: u8,
}

impl OldDirent {
    pub fn new(bytes: Vec<u8>) -> R<OldDirent> {
        let length = bytes.len();
        let (d_ino, d_off, d_reclen) = bytes_to_similar_pieces(&bytes);
        assert!(length == d_reclen as usize);
        let d_name = get_name_from_bytes(&bytes, 18);
        let d_type = bytes[bytes.len() - 1];

        Ok(OldDirent {
            d_ino,
            d_off,
            d_reclen,
            d_name,
            pad: 0,
            d_type,
        })
    }

    pub fn new_with_name(name: Vec<u8>) -> OldDirent {
        let d_reclen = (((name.len() as f32 - 4.0) / 8.0).ceil() as u16) * 8 + 24;
        OldDirent {
            d_ino: 0,
            d_off: 0,
            d_reclen,
            d_name: name,
            pad: 0,
            d_type: libc::DT_REG,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let padding_size: usize = self.d_reclen as usize - 20 - self.d_name.len();
        let string_padding: Vec<u8> = vec![0; padding_size];
        tracee_memory::cast_to_eight_byte_array(self.d_ino)
            .iter()
            .chain(tracee_memory::cast_to_eight_byte_array(self.d_off).iter())
            .chain(tracee_memory::cast_to_two_byte_array(self.d_reclen).iter())
            .chain(self.d_name.iter())
            .chain(string_padding.iter())
            .chain([0, self.d_type].iter())
            .cloned()
            .collect()
    }
}

fn bytes_to_similar_pieces(bytes: &Vec<u8>) -> (u64, u64, u16) {
    let mut d_ino = [0; 8];
    d_ino.clone_from_slice(&bytes[0..8]);

    let mut d_off = [0; 8];
    d_off.clone_from_slice(&bytes[8..16]);

    let mut d_reclen_bytes = [0; 2];
    d_reclen_bytes.clone_from_slice(&bytes[16..18]);
    let d_reclen = tracee_memory::cast_to_two_byte_word(d_reclen_bytes);

    (
        tracee_memory::cast_to_eight_byte_word(d_ino),
        tracee_memory::cast_to_eight_byte_word(d_off),
        d_reclen,
    )
}

fn get_name_from_bytes(bytes: &Vec<u8>, offset: usize) -> Vec<u8> {
    bytes[offset..bytes.len()]
        .iter()
        .take_while(|byte| **byte != 0)
        .cloned()
        .collect()
}

#[cfg(test)]
mod directory_entry {
    use super::*;

    const FULL_BUFFER: [u8; 344] = [
        176, 2, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 24, 0, 108, 111, 103, 0, 0, 4, 177, 2,
        0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 99, 97, 99, 104, 101, 0, 0, 0, 0, 0, 0,
        0, 0, 4, 2, 0, 0, 0, 0, 0, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 24, 0, 46, 46, 0, 0, 0, 4, 35, 0,
        0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 24, 0, 46, 0, 0, 0, 0, 4, 178, 2, 0, 0, 0, 0, 0,
        0, 80, 0, 0, 0, 0, 0, 0, 0, 24, 0, 108, 105, 98, 0, 0, 4, 179, 2, 0, 0, 0, 0, 0, 0, 96, 0,
        0, 0, 0, 0, 0, 0, 32, 0, 115, 112, 111, 111, 108, 0, 0, 0, 0, 0, 0, 0, 0, 4, 180, 2, 0, 0,
        0, 0, 0, 0, 112, 0, 0, 0, 0, 0, 0, 0, 24, 0, 109, 97, 105, 108, 0, 4, 181, 2, 0, 0, 0, 0,
        0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 24, 0, 116, 109, 112, 0, 0, 4, 182, 2, 0, 0, 0, 0, 0, 0,
        144, 0, 0, 0, 0, 0, 0, 0, 32, 0, 108, 111, 99, 97, 108, 0, 0, 0, 0, 0, 0, 0, 0, 4, 183, 2,
        0, 0, 0, 0, 0, 0, 160, 0, 0, 0, 0, 0, 0, 0, 24, 0, 111, 112, 116, 0, 0, 4, 36, 0, 0, 0, 0,
        0, 0, 0, 176, 0, 0, 0, 0, 0, 0, 0, 24, 0, 114, 117, 110, 0, 0, 10, 184, 2, 0, 0, 0, 0, 0,
        0, 192, 0, 0, 0, 0, 0, 0, 0, 24, 0, 108, 111, 99, 107, 0, 10, 185, 2, 0, 0, 0, 0, 0, 0,
        216, 0, 0, 0, 0, 0, 0, 0, 32, 0, 98, 97, 99, 107, 117, 112, 115, 0, 0, 0, 0, 0, 0, 4,
    ];

    const NEW_BUFFER: [u8; 24] = [
        230, 5, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 24, 0, 4, 108, 111, 103, 0, 4,
    ];

    const OLD_BUFFER: [u8; 24] = [
        176, 2, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 24, 0, 108, 111, 103, 0, 0, 4,
    ];

    mod splitting {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn splits_buffer_into_pieces() -> R<()> {
            assert!(DirectoryEntry::new_collection_from_buffer(&FULL_BUFFER.to_vec())?.len() == 13);
            Ok(())
        }

        #[test]
        fn splits_into_the_right_sizes() -> R<()> {
            assert_eq!(
                DirectoryEntry::new_collection_from_buffer(&FULL_BUFFER.to_vec())?
                    .iter()
                    .map(|dirent| dirent.size())
                    .collect::<Vec<u16>>(),
                vec![24, 32, 24, 24, 24, 32, 24, 24, 32, 24, 24, 24, 32],
            );
            Ok(())
        }

        #[test]
        fn splits_into_the_right_named_files() -> R<()> {
            assert_eq!(
                DirectoryEntry::new_collection_from_buffer(&FULL_BUFFER.to_vec())?
                    .iter()
                    .map(|dirent| dirent.name())
                    .collect::<Vec<String>>(),
                vec![
                    "log", "cache", "..", ".", "lib", "spool", "mail", "tmp", "local", "opt",
                    "run", "lock", "backups",
                ],
            );
            Ok(())
        }
    }

    mod writes {
        use super::*;
        use pretty_assertions::assert_eq;

        mod round_trip {
            use super::*;
            use pretty_assertions::assert_eq;

            fn round_trip(buffer: [u8; 24]) -> R<()> {
                assert_eq!(
                    DirectoryEntry::new(buffer.to_vec())?.to_bytes(),
                    buffer.to_vec(),
                );
                Ok(())
            }

            #[test]
            fn an_old_dirent_to_buffer() -> R<()> {
                round_trip(OLD_BUFFER)
            }

            #[test]
            fn a_new_dirent_to_buffer() -> R<()> {
                let mut non_tainted_buffer = NEW_BUFFER.clone();
                non_tainted_buffer[23] = 0;
                round_trip(non_tainted_buffer)
            }
        }

    }

    mod reads {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn an_old_buffer() -> R<()> {
            match DirectoryEntry::new(OLD_BUFFER.to_vec())? {
                DirectoryEntry::Old(_) => Ok(()),
                DirectoryEntry::New(_) => panic!("expected an old style buffer"),
            }
        }

        #[test]
        fn a_new_buffer() -> R<()> {
            match DirectoryEntry::new(NEW_BUFFER.to_vec())? {
                DirectoryEntry::New(_) => Ok(()),
                DirectoryEntry::Old(_) => panic!("expected a newfangled buffer"),
            }
        }

        #[test]
        fn gives_a_nice_error_when_provided_buffer_is_too_small() -> R<()> {
            assert_eq!(
                DirectoryEntry::new(vec![]).map_err(|err| err.to_string()),
                Err(
                    "the smallest valid 'dirent' struct is 24 bytes, only 0 bytes given"
                        .to_string()
                ),
            );
            Ok(())
        }
    }

    fn test_reclen_boundaries<F>(constructor: F, string: &str, expected_size: u16)
    where
        F: Fn(Vec<u8>) -> DirectoryEntry,
    {
        assert_eq!(
            constructor(string.as_bytes().to_vec()).size(),
            expected_size
        )
    }

    mod new_dirent {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn converts_a_byte_array_to_struct() -> R<()> {
            assert_eq!(
                NewDirent::new(NEW_BUFFER.to_vec())?,
                NewDirent {
                    d_ino: 1510,
                    d_off: 16,
                    d_reclen: 24,
                    d_type: 4,
                    d_name: vec![108, 111, 103],
                }
            );
            Ok(())
        }

        #[test]
        fn sets_record_length_in_increments_of_eight_based_on_name_length() -> R<()> {
            fn new(name: Vec<u8>) -> DirectoryEntry {
                DirectoryEntry::New(NewDirent::new_with_name(name))
            }

            test_reclen_boundaries(new, "asdfa", 24);
            test_reclen_boundaries(new, "asdfas", 32);
            test_reclen_boundaries(new, "asdfasdfasdfa", 32);
            test_reclen_boundaries(new, "asdfasdfasdfas", 40);
            Ok(())
        }
    }

    mod old_dirent {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn converts_a_byte_array_to_struct() -> R<()> {
            assert_eq!(
                OldDirent::new(OLD_BUFFER.to_vec())?,
                OldDirent {
                    d_ino: 688,
                    d_off: 16,
                    d_reclen: 24,
                    d_name: vec![108, 111, 103],
                    pad: 0,
                    d_type: 4,
                }
            );
            Ok(())
        }

        #[test]
        fn sets_record_length_in_increments_of_eight_based_on_name_length() -> R<()> {
            fn old(name: Vec<u8>) -> DirectoryEntry {
                DirectoryEntry::Old(OldDirent::new_with_name(name))
            }

            test_reclen_boundaries(old, "asdf", 24);
            test_reclen_boundaries(old, "asdfa", 32);
            test_reclen_boundaries(old, "asdfasdfasdf", 32);
            test_reclen_boundaries(old, "asdfasdfasdfa", 40);
            Ok(())
        }

    }
}
