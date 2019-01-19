mod hook;
mod process;
mod servermonitor;
mod windowsloop;
use crate::hook::{HookDll, HookId, Library, WindowsHook};
use crate::servermonitor::ServerMonitor;
use std::env;
use std::error::Error;
use std::fmt;
use wintrap::{self, Signal};
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
    EventLoop(WindowsError),
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
            MainError::EventLoop(e) => write!(f, "Error in Windows event loop: {}", e),
        }
    }
}

impl Error for MainError {}

fn run() -> Result<i32, MainError> {
    let main_thread_id = windowsloop::get_current_thread_id();
    wintrap::trap(
        &[Signal::CtrlC, Signal::CloseWindow, Signal::CloseConsole],
        move |_| {
            trace!("Received interrupt");
            windowsloop::post_quit_message(main_thread_id, 1).unwrap();
        },
        || {
            let server_pid = env::var("WLW_PID")
                .map_err(|_| MainError::PidIsMissing)?
                .parse::<u32>()
                .map_err(|_| MainError::PidIsInvalid)?;
            if server_pid == 0 {
                return Err(MainError::PidIs0);
            }
            let dll_path = env::var("WLW_HOOK_DLL").map_err(|_| MainError::HookDllIsMissing)?;
            let library = Library::new(&dll_path).map_err(MainError::DllLoadError)?;
            let hook_dll = HookDll::new(library, server_pid).map_err(MainError::DllHookError)?;
            // Hooks
            let _callwndproc_hook = WindowsHook::new(
                HookId::CallWndProc,
                hook_dll.callwndproc_proc,
                &hook_dll.library,
            );
            let _cbt_hook = WindowsHook::new(HookId::Cbt, hook_dll.cbt_proc, &hook_dll.library);
            // Monitor the server process to ensure it remains active
            let _monitor = ServerMonitor::new(server_pid, move |e| {
                error!("Server connection seems to have failed: {}", e);
                windowsloop::post_quit_message(main_thread_id, 1).unwrap();
            });
            // Event loop
            let rc = windowsloop::run_event_loop().map_err(MainError::EventLoop)?;
            Ok(rc)
        },
    )
    .unwrap()
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
        Ok(rc) => std::process::exit(rc),
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    }
}
