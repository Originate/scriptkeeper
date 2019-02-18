use crate::R;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ShortTempFile {
    pub path: PathBuf,
}

impl ShortTempFile {
    pub fn new(contents: &[u8]) -> R<ShortTempFile> {
        ShortTempFile::new_internal(&PathBuf::from("/tmp"), contents)
    }

    fn new_internal(directory: &Path, contents: &[u8]) -> R<ShortTempFile> {
        let (path, mut file) = ShortTempFile::create_new_file(directory)?;
        file.write_all(contents)?;
        file.flush()?;
        Ok(ShortTempFile { path })
    }

    fn create_new_file(directory: &Path) -> R<(PathBuf, File)> {
        let mut result = None;
        for name in Names::new() {
            let path = directory.join(name);
            match ShortTempFile::try_create_new_file(&path)? {
                Some(file) => {
                    result = Some((path, file));
                    break;
                }
                None => {}
            }
        }
        Ok(match result {
            Some(result) => result,
            None => Err("short_temp_files: ran out of temporary file names")?,
        })
    }

    fn try_create_new_file(path: &Path) -> R<Option<File>> {
        Ok(
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o777)
                .open(&path)
            {
                Ok(file) => Some(file),
                Err(error) => match error.kind() {
                    io::ErrorKind::AlreadyExists => None,
                    _ => Err(error)?,
                },
            },
        )
    }
}

impl Drop for ShortTempFile {
    fn drop(&mut self) {
        fs::remove_file(&self.path).unwrap_or_else(|_| {
            panic!(
                "short_temp_files: failed to remove {}",
                self.path.to_string_lossy()
            );
        });
    }
}

#[cfg(test)]
mod short_temp_files {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempdir::TempDir;

    #[test]
    fn creates_a_temporary_file_with_the_given_contents() -> R<()> {
        let tempdir = TempDir::new("test")?;
        let tempfile = ShortTempFile::new_internal(tempdir.path(), b"foo")?;
        assert_eq!(fs::read(&tempfile.path)?, b"foo");
        Ok(())
    }

    #[test]
    fn new_puts_files_into_tmp() -> R<()> {
        let tempfile = ShortTempFile::new(b"foo")?;
        assert_eq!(tempfile.path.parent().unwrap(), PathBuf::from("/tmp"));
        Ok(())
    }

    #[test]
    fn uses_free_file_names() -> R<()> {
        let tempdir = TempDir::new("test")?;
        fs::write(tempdir.path().join("a"), "existing")?;
        let tempfile = ShortTempFile::new_internal(tempdir.path(), b"new")?;
        assert_eq!(
            String::from_utf8(fs::read(tempdir.path().join("a"))?)?,
            "existing"
        );
        assert_eq!(String::from_utf8(fs::read(&tempfile.path)?)?, "new");
        Ok(())
    }

    #[test]
    fn sets_the_executable_flag() -> R<()> {
        let tempdir = TempDir::new("test")?;
        let tempfile = ShortTempFile::new_internal(tempdir.path(), b"#!/usr/bin/env bash\ntrue")?;
        assert_eq!(Command::new(&tempfile.path).status()?.code(), Some(0));
        Ok(())
    }

    #[test]
    fn removes_files_on_drop() -> R<()> {
        let tempdir = TempDir::new("test")?;
        let tempfile = ShortTempFile::new_internal(tempdir.path(), b"foo")?;
        let path = tempfile.path.to_path_buf();
        std::mem::drop(tempfile);
        assert!(!path.exists(), format!("still exists: {:?}", path));
        Ok(())
    }
}

struct Names {
    a: Box<dyn Iterator<Item = char>>,
    old_a: char,
    b: Box<dyn Iterator<Item = char>>,
}

impl Names {
    fn new() -> Names {
        let mut a_iter = Names::mk_digit_iter();
        let old_a = a_iter.next().unwrap();
        Names {
            a: a_iter,
            old_a,
            b: Names::mk_digit_iter(),
        }
    }

    fn mk_digit_iter() -> Box<impl Iterator<Item = char>> {
        Box::new("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890-_+=".chars())
    }
}

impl Iterator for Names {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.b.next() {
            Some(b) => Some(vec![self.old_a, b].into_iter().collect()),
            None => match self.a.next() {
                Some(a) => {
                    self.old_a = a;
                    self.b = Names::mk_digit_iter();
                    self.next()
                }
                None => None,
            },
        }
    }
}

#[cfg(test)]
mod names {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn generates_a_lot_of_names() {
        assert_eq!(Names::new().count(), 4356);
    }

    #[test]
    fn generates_unique_names() {
        let mut set = HashSet::new();
        for name in Names::new() {
            set.insert(name);
        }
        assert_eq!(set.len(), Names::new().count());
    }

    #[test]
    fn generates_names_below_three_characters() {
        for name in Names::new() {
            assert!(name.len() < 3);
        }
    }
}
