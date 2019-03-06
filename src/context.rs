use crate::R;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Context {
    Context {
        check_protocols_executable: PathBuf,
    },
    #[cfg(feature = "test")]
    TestContext,
}

impl Context {
    pub fn new() -> R<Context> {
        Ok(Context::Context {
            check_protocols_executable: std::env::current_exe()?,
        })
    }

    #[cfg(feature = "test")]
    pub fn new_mock() -> Context {
        Context::TestContext
    }

    pub fn check_protocols_executable(&self) -> PathBuf {
        match self {
            Context::Context {
                check_protocols_executable,
            } => check_protocols_executable.clone(),
            #[cfg(feature = "test")]
            Context::TestContext => {
                let cwd = std::env::current_dir().unwrap();
                cwd.join("./target/debug/check-protocols")
            }
        }
    }
}
