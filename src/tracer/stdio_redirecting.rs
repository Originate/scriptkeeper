use crate::context::Context;
use crate::R;
use libc::c_int;
use nix::unistd::{close, dup2, pipe, read};
use std::fmt;
use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};
use std::thread;

type RawFd = c_int;

pub struct Redirector {
    pub stdout: Redirect,
    pub stderr: Redirect,
}

pub struct Capture {
    pub stdout: bool,
    pub stderr: bool,
}

impl Redirector {
    pub fn new(context: &Context, capture: Capture) -> R<Redirector> {
        Ok(Redirector {
            stdout: if capture.stdout {
                Redirect::new_capturing(context, StreamType::Stdout)?
            } else {
                Redirect::new_non_capturing(context, StreamType::Stdout)?
            },
            stderr: if capture.stderr {
                Redirect::new_capturing(context, StreamType::Stderr)?
            } else {
                Redirect::new_non_capturing(context, StreamType::Stderr)?
            },
        })
    }

    pub fn child_redirect_streams(&self) -> R<()> {
        self.stdout.child_redirect_stream()?;
        self.stderr.child_redirect_stream()?;
        Ok(())
    }

    pub fn parent_relay_streams(&self) -> R<impl FnOnce() -> R<()>> {
        let a = self.stdout.parent_relay_stream()?;
        let b = self.stderr.parent_relay_stream()?;
        Ok(|| -> R<()> {
            a.join().unwrap()?;
            b.join().unwrap()?;
            Ok(())
        })
    }
}

#[derive(Clone, Copy)]
pub enum StreamType {
    Stdout,
    Stderr,
}

impl fmt::Display for StreamType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StreamType::Stdout => write!(f, "stdout"),
            StreamType::Stderr => write!(f, "stderr"),
        }
    }
}

pub struct Redirect {
    pub stream_type: StreamType,
    context: Context,
    read_end: RawFd,
    write_end: RawFd,
    captured: Option<Arc<Mutex<Cursor<Vec<u8>>>>>,
}

impl Redirect {
    fn new_capturing(context: &Context, stream_type: StreamType) -> R<Redirect> {
        Redirect::new(
            context,
            stream_type,
            Some(Arc::new(Mutex::new(Cursor::new(vec![])))),
        )
    }

    fn new_non_capturing(context: &Context, stream_type: StreamType) -> R<Redirect> {
        Redirect::new(context, stream_type, None)
    }

    fn new(
        context: &Context,
        stream_type: StreamType,
        captured: Option<Arc<Mutex<Cursor<Vec<u8>>>>>,
    ) -> R<Redirect> {
        let (read_end, write_end) = pipe()?;
        Ok(Redirect {
            stream_type,
            context: context.clone(),
            read_end,
            write_end,
            captured,
        })
    }

    fn child_redirect_stream(&self) -> R<()> {
        close(self.read_end)?;
        let stdstream_fileno = match self.stream_type {
            StreamType::Stdout => libc::STDOUT_FILENO,
            StreamType::Stderr => libc::STDERR_FILENO,
        };
        dup2(self.write_end, stdstream_fileno)?;
        close(self.write_end)?;
        Ok(())
    }

    fn parent_relay_stream(&self) -> R<thread::JoinHandle<Result<(), String>>> {
        close(self.write_end)?;
        let read_end = self.read_end;
        let context = self.context.clone();
        let stream_type = self.stream_type;
        let captured = self.captured.clone();
        Ok(thread::spawn(move || -> Result<(), String> {
            let mut buffer = [0; 1024];
            loop {
                let count = read(read_end, &mut buffer).map_err(|error| error.to_string())?;
                if count == 0 {
                    return Ok(());
                }
                let mut stdstream = match stream_type {
                    StreamType::Stdout => context.stdout(),
                    StreamType::Stderr => context.stderr(),
                };
                stdstream
                    .write_all(&buffer[..count])
                    .map_err(|error| error.to_string())?;
                if let Some(captured) = &captured {
                    captured
                        .lock()
                        .unwrap()
                        .write_all(&buffer[..count])
                        .unwrap();
                }
            }
        }))
    }

    pub fn captured(&self) -> R<Option<Vec<u8>>> {
        Ok(match &self.captured {
            None => None,
            Some(captured) => Some({
                let cursor = captured.lock().map_err(|error| error.to_string())?;
                cursor.clone().into_inner()
            }),
        })
    }
}
