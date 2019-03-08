use crate::R;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Context {
    Context {
        check_protocols_executable: PathBuf,
    },
    #[cfg(feature = "test")]
    TestContext {
        stderr: mock_stderr::MockStderr,
    },
}

impl Context {
    pub fn new() -> R<Context> {
        Ok(Context::Context {
            check_protocols_executable: std::env::current_exe()?,
        })
    }

    #[cfg(feature = "test")]
    pub fn new_mock() -> Context {
        Context::TestContext {
            stderr: mock_stderr::MockStderr::default(),
        }
    }

    pub fn check_protocols_executable(&self) -> PathBuf {
        match self {
            Context::Context {
                check_protocols_executable,
            } => check_protocols_executable.clone(),
            #[cfg(feature = "test")]
            Context::TestContext { .. } => {
                let cwd = std::env::current_dir().unwrap();
                cwd.join("./target/debug/check-protocols")
            }
        }
    }

    pub fn stderr(&self) -> Box<Write> {
        match self {
            Context::Context { .. } => Box::new(std::io::stderr()),
            #[cfg(feature = "test")]
            Context::TestContext { stderr } => Box::new(stderr.clone()),
        }
    }

    #[cfg(feature = "test")]
    pub fn get_captured_stderr(&self) -> String {
        match self {
            Context::Context { .. } => panic!("tests should use the TestContext"),
            Context::TestContext { stderr } => stderr.get_captured_stderr(),
        }
    }
}

#[cfg(feature = "test")]
mod mock_stderr {
    use std::io::{Cursor, Error, ErrorKind, Write};
    use std::sync::{Arc, Mutex, MutexGuard};

    #[derive(Debug, Clone)]
    pub struct MockStderr {
        cursor: Arc<Mutex<Cursor<Vec<u8>>>>,
    }

    impl MockStderr {
        pub fn get_captured_stderr(&self) -> String {
            let cursor = self.cursor.lock().unwrap();
            String::from_utf8(cursor.clone().into_inner()).unwrap()
        }

        fn lock(&self) -> Result<MutexGuard<Cursor<Vec<u8>>>, Error> {
            Ok(self
                .cursor
                .lock()
                .map_err(|error| std::io::Error::new(ErrorKind::Other, error.to_string()))?)
        }
    }

    impl Default for MockStderr {
        fn default() -> MockStderr {
            MockStderr {
                cursor: Arc::new(Mutex::new(Cursor::new(vec![]))),
            }
        }
    }

    impl Write for MockStderr {
        fn write(&mut self, chunk: &[u8]) -> Result<usize, Error> {
            self.lock()?.write(chunk)
        }

        fn flush(&mut self) -> Result<(), Error> {
            self.lock()?.flush()
        }
    }
}
