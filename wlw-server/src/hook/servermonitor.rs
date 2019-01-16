use crate::process::Process;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use wlw_server::mainwindow::MainWindow;
use wlw_server::windowserror::WindowsError;

#[derive(Debug)]
pub enum MonitorError {
    ProcessHandleAquisition(WindowsError),
    ServerSync(WindowsError),
    ServerQuit(u32),
}

pub struct ServerMonitor {
    thread_run: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<Result<(), MonitorError>>>,
    stopped: bool,
}

impl fmt::Display for MonitorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MonitorError::ProcessHandleAquisition(e) => {
                write!(f, "Error acquiring handle to server process: {}", e)
            }
            MonitorError::ServerSync(e) => {
                write!(f, "Error retrieving server process status: {}", e)
            }
            MonitorError::ServerQuit(code) => {
                write!(f, "Server exited prematurely with code {}", code)
            }
        }
    }
}

impl Error for MonitorError {}

impl ServerMonitor {
    pub fn new(pid: u32, mw: Arc<MainWindow>) -> Self {
        let thread_run = Arc::new(AtomicBool::new(true));
        let run = thread_run.clone();
        let thread = Some(thread::spawn(move || -> Result<(), MonitorError> {
            let process = match Process::new(pid) {
                Ok(process) => process,
                Err(e) => {
                    mw.close();
                    return Err(MonitorError::ProcessHandleAquisition(e));
                }
            };
            while run.load(Ordering::Relaxed) {
                match process.try_wait() {
                    Ok(Some(code)) => {
                        mw.close();
                        return Err(MonitorError::ServerQuit(code));
                    }
                    Err(e) => {
                        mw.close();
                        return Err(MonitorError::ServerSync(e));
                    }
                    Ok(None) => {}
                }
                thread::sleep(Duration::from_millis(500));
            }
            Ok(())
        }));
        ServerMonitor {
            thread_run,
            thread,
            stopped: false,
        }
    }

    pub fn stop(mut self) -> Result<(), MonitorError> {
        self.stop_mut_ref()
    }

    fn stop_mut_ref(&mut self) -> Result<(), MonitorError> {
        self.stopped = true;
        self.thread_run.store(false, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap()
    }
}

impl Drop for ServerMonitor {
    fn drop(&mut self) {
        if !self.stopped {
            self.stop_mut_ref().unwrap();
        }
    }
}
