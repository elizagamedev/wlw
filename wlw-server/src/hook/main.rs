mod hook;
mod process;
mod servermonitor;
use crate::hook::{HookDll, HookId, Library, WindowsHook};
use crate::servermonitor::{MonitorError, ServerMonitor};
use ctrlc;
use std::env;
use std::error::Error;
use std::fmt;
use wlw_server::mainwindow::MainWindow;
use wlw_server::windowserror::WindowsError;
#[macro_use]
extern crate log;
use flexi_logger::Logger;

#[derive(Debug)]
enum MainError {
    PidIsMissing,
    PidIsInvalid,
    PidIs0,
    HookDllIsMissing,
    DllLoadError(WindowsError),
    DllHookError(WindowsError),
    MainWindowError(WindowsError),
    CtrlCError,
    EventLoop(WindowsError),
    Monitor(MonitorError),
}

impl fmt::Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MainError::PidIsMissing => write!(f, "Must set WLW_PID"),
            MainError::PidIsInvalid => write!(f, "WLW_PID is not a valid number"),
            MainError::PidIs0 => write!(f, "WLW_PID cannot be 0"),
            MainError::HookDllIsMissing => write!(f, "WLW_HOOK_DLL must be set"),
            MainError::DllLoadError(e) => write!(f, "DLL load error: {}", e),
            MainError::DllHookError(e) => write!(f, "Windows hook error: {}", e),
            MainError::MainWindowError(e) => write!(f, "Window creation error: {}", e),
            MainError::CtrlCError => write!(f, "Could not set Ctrl-C handler"),
            MainError::EventLoop(e) => write!(f, "Error in Windows event loop: {}", e),
            MainError::Monitor(e) => write!(f, "{}", e),
        }
    }
}

impl Error for MainError {}

fn run() -> Result<(), MainError> {
    let server_pid = match env::var("WLW_PID") {
        Ok(pid_str) => match pid_str.parse::<u32>() {
            Ok(pid) => {
                if pid == 0 {
                    Err(MainError::PidIs0)
                } else {
                    Ok(pid)
                }
            }
            Err(_) => Err(MainError::PidIsInvalid),
        },
        Err(_) => Err(MainError::PidIsMissing),
    }?;
    let dll_path = match env::var("WLW_HOOK_DLL") {
        Ok(path) => Ok(path),
        Err(_) => Err(MainError::HookDllIsMissing),
    }?;
    let library = match Library::new(&dll_path) {
        Ok(o) => Ok(o),
        Err(e) => Err(MainError::DllLoadError(e)),
    }?;
    let hook_dll = match HookDll::new(library, server_pid) {
        Ok(o) => Ok(o),
        Err(e) => Err(MainError::DllHookError(e)),
    }?;
    let mw = match MainWindow::new("wlw_hook", Box::new(|_, _, _, _| false)) {
        Ok(o) => Ok(o),
        Err(e) => Err(MainError::MainWindowError(e)),
    }?;
    // Ctrl-C handler
    let ctrlc_mw = mw.clone();
    match ctrlc::set_handler(move || ctrlc_mw.close()) {
        Ok(_) => Ok(()),
        Err(_) => Err(MainError::CtrlCError),
    }?;
    // Monitor the server process to ensure it remains active
    let mut monitor = ServerMonitor::new(server_pid, mw.clone());
    // Hooks
    let _callwndproc_hook = WindowsHook::new(
        HookId::CallWndProc,
        hook_dll.callwndproc_proc,
        &hook_dll.library,
    );
    let _cbt_hook = WindowsHook::new(HookId::Cbt, hook_dll.cbt_proc, &hook_dll.library);
    // Event loop
    match mw.run_event_loop() {
        Ok(_) => Ok(()),
        Err(e) => Err(MainError::EventLoop(e)),
    }?;
    // Stop server monitor thread
    match monitor.stop() {
        Ok(_) => Ok(()),
        Err(e) => Err(MainError::Monitor(e)),
    }?;
    Ok(())
}

fn main() {
    Logger::with_env_or_str("trace")
        .format(|w, record| {
            #[cfg(target_pointer_width = "32")]
            static WIDTH: &str = "32";
            #[cfg(target_pointer_width = "64")]
            static WIDTH: &str = "64";
            write!(
                w,
                "HOOK{}:{} [{}] {}",
                WIDTH,
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
