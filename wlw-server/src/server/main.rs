#[macro_use]
extern crate log;
#[cfg(debug_assertions)]
use failure::Error;
mod debug;
mod hookmanager;
mod pipeserver;
use crate::hookmanager::HookManager;
use crate::pipeserver::PipeServer;
use ctrlc;
use flexi_logger::Logger;
use std::error;
use std::fmt;
use wlw_server::mainwindow::MainWindow;
use wlw_server::windowserror::WindowsError;

#[derive(Debug)]
enum MainError {
    MainWindowError(WindowsError),
    CtrlCError,
    EventLoop(WindowsError),
    PipeServerInit(WindowsError),
    PipeServerError(Error),
}

impl fmt::Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MainError::MainWindowError(e) => write!(f, "Window creation error: {}", e),
            MainError::CtrlCError => write!(f, "Could not set Ctrl-C handler"),
            MainError::EventLoop(e) => write!(f, "Error in Windows event loop: {}", e),
            MainError::PipeServerInit(e) => write!(f, "Error creating pipe server: {}", e),
            MainError::PipeServerError(e) => write!(f, "Pipe server error: {}", e),
        }
    }
}

impl error::Error for MainError {}

fn run() -> Result<(), MainError> {
    let mw = match MainWindow::new("wlw_server", Box::new(|_, _, _, _| false)) {
        Ok(o) => Ok(o),
        Err(e) => Err(MainError::MainWindowError(e)),
    }?;
    // Ctrl-C handler
    let ctrlc_mw = mw.clone();
    match ctrlc::set_handler(move || ctrlc_mw.close()) {
        Ok(_) => Ok(()),
        Err(_) => Err(MainError::CtrlCError),
    }?;
    // Pipe server
    let pipe_name = format!("wlw_server_{}", std::process::id());
    let ps_mw = mw.clone();
    let ps = match PipeServer::new(
        pipe_name,
        Box::new(|_| Ok(())),
        Some(Box::new(move || ps_mw.close())),
    ) {
        Ok(o) => Ok(o),
        Err(e) => Err(MainError::PipeServerInit(e)),
    }?;
    // Hook manager
    let _hm = HookManager::new();
    // Event loop
    match mw.run_event_loop() {
        Ok(_) => Ok(()),
        Err(e) => Err(MainError::EventLoop(e)),
    }?;
    // Stop pipe server
    match ps.stop() {
        Ok(_) => Ok(()),
        Err(e) => Err(MainError::PipeServerError(e)),
    }?;
    Ok(())
}

fn main() {
    Logger::with_env_or_str("trace")
        .format(|w, record| {
            write!(
                w,
                "SERVER:{} [{}] {}",
                record.level(),
                record.module_path().unwrap_or("<unnamed>"),
                record.args()
            )
        })
        .start()
        .unwrap();
    match run() {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    }
}
