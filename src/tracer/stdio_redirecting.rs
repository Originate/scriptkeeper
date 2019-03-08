use crate::context::Context;
use crate::R;
use libc::c_int;
use nix::unistd::{close, dup2, pipe, read};
use std::thread;

type RawFd = c_int;

pub struct Redirector {
    context: Context,
    read_end: RawFd,
    write_end: RawFd,
}

impl Redirector {
    pub fn new(context: &Context) -> R<Redirector> {
        let (read_end, write_end) = pipe()?;
        Ok(Redirector {
            context: context.clone(),
            read_end,
            write_end,
        })
    }

    pub fn child(&self) -> R<()> {
        close(self.read_end)?;
        dup2(self.write_end, libc::STDERR_FILENO)?;
        close(self.write_end)?;
        Ok(())
    }

    pub fn parent(&self) -> R<()> {
        close(self.write_end)?;
        let read_end = self.read_end;
        let context = self.context.clone();
        thread::spawn(move || {
            let mut buffer = [0; 1024];
            loop {
                match read(read_end, &mut buffer) {
                    Ok(count) => {
                        if count == 0 {
                            break;
                        }
                        context.stderr().write_all(&buffer[..count]).unwrap();
                    }
                    Err(error) => {
                        context
                            .stderr()
                            .write_all(format!("{}", error).as_bytes())
                            .unwrap();
                        break;
                    }
                }
            }
        });
        Ok(())
    }
}
