use quale::which;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

pub fn compare_executables(a: &[u8], b: &[u8]) -> bool {
    canonicalize(a) == canonicalize(b)
}

#[cfg(test)]
mod compare_executables {
    use super::*;
    use crate::R;

    #[test]
    fn returns_true_if_executables_are_identical() -> R<()> {
        let executable = b"./bin/myexec";
        assert!(compare_executables(executable, executable));
        Ok(())
    }

    #[test]
    fn returns_false_if_executables_are_distinct() -> R<()> {
        let a = b"./bin/myexec";
        let b = b"./bin/myotherexec";
        assert!(!compare_executables(a, b));
        Ok(())
    }

    #[test]
    fn returns_true_if_executables_match_after_lookup_in_path() -> R<()> {
        let path = which("cp").unwrap();
        let cp_long = path.as_os_str().as_bytes();
        let cp_short = b"cp";
        assert!(compare_executables(cp_long, cp_short));
        Ok(())
    }
}

pub fn canonicalize(executable: &[u8]) -> Vec<u8> {
    let path = PathBuf::from(OsStr::from_bytes(executable));
    let file_name = match path.file_name() {
        None => return executable.to_vec(),
        Some(f) => f,
    };
    match which(file_name) {
        Some(resolved) => {
            if resolved == path {
                file_name.as_bytes()
            } else {
                executable
            }
        }
        None => executable,
    }
    .to_vec()
}

#[cfg(test)]
mod canonicalize {
    use super::*;
    use crate::R;
    use pretty_assertions::assert_eq;

    #[test]
    fn shortens_absolute_executable_paths_if_found_in_path() -> R<()> {
        let executable = "cp";
        let resolved = which(executable).unwrap();
        let file_name = canonicalize(resolved.as_os_str().as_bytes());
        assert_eq!(String::from_utf8(file_name)?, "cp");
        Ok(())
    }

    #[test]
    fn does_not_shorten_executable_that_is_not_in_path() -> R<()> {
        let executable = b"/foo/doesnotexist";
        let file_name = canonicalize(executable);
        assert_eq!(String::from_utf8(file_name)?, "/foo/doesnotexist");
        Ok(())
    }

    #[test]
    fn does_not_shorten_executable_that_is_not_in_path_but_has_same_name_as_one_that_is() -> R<()> {
        let executable = b"/not/in/path/ls";
        let file_name = canonicalize(executable);
        assert_eq!(String::from_utf8(file_name)?, "/not/in/path/ls");
        Ok(())
    }

    #[test]
    fn does_not_shorten_relative_path() -> R<()> {
        let executable = b"./foo";
        let file_name = canonicalize(executable);
        assert_eq!(String::from_utf8(file_name)?, "./foo");
        Ok(())
    }

    #[test]
    fn does_not_modify_short_forms_if_found_in_path() -> R<()> {
        let executable = b"ls";
        let file_name = canonicalize(executable);
        assert_eq!(String::from_utf8(file_name)?, "ls");
        Ok(())
    }
}
