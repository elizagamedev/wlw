#[macro_use]
extern crate log;
#[cfg(debug_assertions)]
use wintrap::{self, Signal};
mod debug;
mod hookmanager;
mod pipeserver;
use crate::hookmanager::HookManager;
use crate::pipeserver::PipeServer;
use flexi_logger::Logger;
use std::error;
use std::fmt;

#[derive(Debug)]
enum MainError {
    PipeServerInit(pipeserver::Error),
}

impl fmt::Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MainError::PipeServerInit(e) => write!(f, "Error creating pipe server: {}", e),
        }
    }
}

impl error::Error for MainError {}

fn run() -> Result<(), MainError> {
    // Pipe server
    let pipe_name = format!("wlw_server_{}", std::process::id());
    let _ps = PipeServer::new(pipe_name, |_: pipeserver::Request<usize, usize>| {}, |_| {})
        .map_err(MainError::PipeServerInit)?;
    // Hook manager
    let _hm = HookManager::new();
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
