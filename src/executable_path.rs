use quale::which;
use std::path::{Path, PathBuf};

pub fn compare_executables(a: &Path, b: &Path) -> bool {
    canonicalize(a) == canonicalize(b)
}

#[cfg(test)]
mod compare_executables {
    use super::*;
    use crate::R;

    #[test]
    fn returns_true_if_executables_are_identical() -> R<()> {
        let executable = Path::new("./bin/myexec");
        assert!(compare_executables(executable, executable));
        Ok(())
    }

    #[test]
    fn returns_false_if_executables_are_distinct() -> R<()> {
        let a = Path::new("./bin/myexec");
        let b = Path::new("./bin/myotherexec");
        assert!(!compare_executables(a, b));
        Ok(())
    }

    #[test]
    fn returns_true_if_executables_match_after_lookup_in_path() -> R<()> {
        let path = which("cp").unwrap();
        let cp_long = path;
        let cp_short = Path::new("cp");
        assert!(compare_executables(&cp_long, cp_short));
        Ok(())
    }

    #[test]
    fn returns_true_for_non_existing_executables_in_bin() -> R<()> {
        let cp_long = Path::new("/bin/doesnotexist");
        let cp_short = Path::new("doesnotexist");
        assert!(compare_executables(&cp_long, cp_short));
        Ok(())
    }
}

pub fn canonicalize(executable: &Path) -> PathBuf {
    let file_name = match executable.file_name() {
        None => return executable.into(),
        Some(f) => f,
    };
    match which(file_name) {
        Some(resolved) => {
            if resolved == executable {
                PathBuf::from(file_name)
            } else {
                executable.into()
            }
        }
        None => {
            if should_assume_in_path(executable) {
                PathBuf::from(file_name)
            } else {
                executable.into()
            }
        }
    }
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
        let file_name = canonicalize(&resolved);
        assert_eq!(file_name, PathBuf::from("cp"));
        Ok(())
    }

    #[test]
    fn does_not_shorten_executable_that_is_not_in_path() -> R<()> {
        let executable = Path::new("/foo/doesnotexist");
        let file_name = canonicalize(executable);
        assert_eq!(file_name, PathBuf::from("/foo/doesnotexist"));
        Ok(())
    }

    #[test]
    fn does_not_shorten_executable_that_is_not_in_path_but_has_same_name_as_one_that_is() -> R<()> {
        let executable = Path::new("/not/in/path/ls");
        let file_name = canonicalize(executable);
        assert_eq!(file_name, PathBuf::from("/not/in/path/ls"));
        Ok(())
    }

    #[test]
    fn does_not_shorten_relative_path() -> R<()> {
        let executable = Path::new("./foo");
        let file_name = canonicalize(executable);
        assert_eq!(file_name, PathBuf::from("./foo"));
        Ok(())
    }

    #[test]
    fn does_not_modify_short_forms_if_found_in_path() -> R<()> {
        let executable = Path::new("ls");
        let file_name = canonicalize(executable);
        assert_eq!(file_name, PathBuf::from("ls"));
        Ok(())
    }

    mod executables_that_do_not_exist {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn shortens_executables_in_bin_that_do_not_exist() -> R<()> {
            let executable = Path::new("/bin/doesnotexist");
            let file_name = canonicalize(executable);
            assert_eq!(file_name, PathBuf::from("doesnotexist"));
            Ok(())
        }

        #[test]
        fn does_not_shorten_executables_in_subdirectory_of_bin_that_do_not_exist() -> R<()> {
            let executable = Path::new("/bin/sub/doesnotexist");
            let file_name = canonicalize(executable);
            assert_eq!(file_name, PathBuf::from("/bin/sub/doesnotexist"));
            Ok(())
        }
    }
}

pub fn should_assume_in_path(executable: &Path) -> bool {
    let file_name = match executable.file_name() {
        None => return false,
        Some(f) => f,
    };
    executable.starts_with("/bin/") && PathBuf::from("/bin/").join(file_name) == executable
}

pub fn is_unmocked_command(unmocked_commands: &[PathBuf], executable: &Path) -> bool {
    unmocked_commands
        .iter()
        .any(|unmocked_command| compare_executables(unmocked_command, &executable))
}
