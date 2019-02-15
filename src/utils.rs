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
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempdir::TempDir;

    fn run(command: &str, args: Vec<&str>) -> R<()> {
        let status = Command::new(command).args(args).status()?;
        if status.success() {
            Ok(())
        } else {
            Err("command failed")?
        }
    }

    pub struct TempFile {
        tempdir: TempDir,
    }

    impl TempFile {
        pub fn new() -> R<TempFile> {
            let tempdir = TempDir::new("test")?;
            Ok(TempFile { tempdir })
        }

        pub fn write_temp_script(script: &str) -> R<TempFile> {
            let tempfile = TempFile::new()?;
            fs::write(&tempfile.path(), script.trim_start())?;
            run("chmod", vec!["+x", tempfile.path().to_str().unwrap()])?;
            Ok(tempfile)
        }

        pub fn path(&self) -> PathBuf {
            self.tempdir.path().join("file")
        }
    }

}
