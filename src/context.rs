use crate::R;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Context {
    pub check_protocols_executable: PathBuf,
}

impl Context {
    pub fn new() -> R<Context> {
        Ok(Context {
            check_protocols_executable: std::env::current_exe()?,
        })
    }

    pub fn new_test_context() -> Context {
        let cwd = std::env::current_dir().unwrap();
        Context {
            check_protocols_executable: cwd.join("./target/debug/check-protocols"),
        }
    }
}
