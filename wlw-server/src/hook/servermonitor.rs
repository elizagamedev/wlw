use crate::process::Process;
use std::error;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use wlw_server::windows;

#[derive(Debug)]
pub enum Error {
    ProcessHandleAquisition(windows::Error),
    ServerSync(windows::Error),
    ServerQuit(u32),
}

pub struct ServerMonitor {
    thread_run: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ProcessHandleAquisition(e) => {
                write!(f, "Error acquiring handle to server process: {}", e)
            }
            Error::ServerSync(e) => write!(f, "Error retrieving server process status: {}", e),
            Error::ServerQuit(code) => write!(f, "Server exited prematurely with code {}", code),
        }
    }
}

impl error::Error for Error {}

impl ServerMonitor {
    pub fn new(pid: u32, on_fail: impl FnOnce(Error) + Send + 'static) -> Self {
        let thread_run = Arc::new(AtomicBool::new(true));
        let run = thread_run.clone();
        let thread = Some(thread::spawn(move || {
            let run = || -> Result<(), Error> {
                let process = Process::new(pid).map_err(Error::ProcessHandleAquisition)?;
                while run.load(Ordering::Relaxed) {
                    match process.try_wait() {
                        Ok(Some(code)) => return Err(Error::ServerQuit(code)),
                        Err(e) => return Err(Error::ServerSync(e)),
                        Ok(None) => {}
                    }
                    thread::sleep(Duration::from_millis(500));
                }
                Ok(())
            };
            if let Err(e) = run() {
                on_fail(e);
            }
        }));
        ServerMonitor { thread_run, thread }
    }
}

impl Drop for ServerMonitor {
    fn drop(&mut self) {
        self.thread_run.store(false, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
