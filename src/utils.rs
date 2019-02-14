use crate::R;
use std::path::Path;

pub fn path_to_string(path: &Path) -> R<&str> {
    Ok(path
        .to_str()
        .ok_or_else(|| format!("invalid utf8 sequence: {:?}", &path))?)
}

#[cfg(test)]
pub mod testing {
    use crate::R;
    use std::path::PathBuf;
    use tempdir::TempDir;

    pub struct TempFile {
        tempdir: TempDir,
    }

    impl TempFile {
        pub fn new() -> R<TempFile> {
            let tempdir = TempDir::new("test")?;
            Ok(TempFile { tempdir })
        }

        pub fn path(&self) -> PathBuf {
            self.tempdir.path().join("file")
        }
    }
}
