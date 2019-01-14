#[cfg(target_pointer_width = "32")]
const POINTER_WIDTH: &str = "32";
#[cfg(target_pointer_width = "64")]
const POINTER_WIDTH: &str = "64";

use ctrlc;
use std::env;
use std::path::Path;
use std::sync::Arc;
use wlw_server::hook::{HookDll, HookId, Library, WindowsHook};
use wlw_server::mainwindow::MainWindow;
#[macro_use]
extern crate log;
use simple_logger;

fn main() {
    simple_logger::init().unwrap();
    std::process::exit(|| -> i32 {
        let server_pid = match env::var("WLW_PID") {
            Ok(o) => match o.parse::<u32>() {
                Ok(o) => o,
                Err(_) => {
                    error!("WLW_PID is not a valid number");
                    return 1;
                }
            },
            Err(_) => {
                error!("Must set WLW_PID to server PID");
                return 1;
            }
        };
        if server_pid == 0 {
            error!("wLW_PID cannot be 0");
            return 1;
        }
        let dll_path = match env::var(format!("WLW_HOOK_DLL_{}", POINTER_WIDTH)) {
            Ok(o) => o,
            Err(_) => {
                error!(
                    "Must set WLW_HOOK_DLL_{} to the path of the hook DLL",
                    POINTER_WIDTH
                );
                return 1;
            }
        };
        let dll_path = Path::new(&dll_path);
        let library = match Library::new(&dll_path) {
            Ok(o) => o,
            Err(e) => {
                error!("Error loading hook DLL: {}", e);
                return 1;
            }
        };
        let hook_dll = match HookDll::new(library, server_pid) {
            Ok(o) => o,
            Err(e) => {
                error!("Error loading hook DLL procs: {}", e);
                return 1;
            }
        };
        let mw = match MainWindow::new(&format!("wlw_hook{}", POINTER_WIDTH), |_, _, _, _| false) {
            Ok(o) => Arc::new(o),
            Err(e) => {
                error!("Error creating main window: {}", e);
                return 1;
            }
        };
        // Hooks
        let _callwndproc_hook = WindowsHook::new(
            HookId::CallWndProc,
            hook_dll.callwndproc_proc,
            &hook_dll.library,
        );
        let _cbt_hook = WindowsHook::new(HookId::Cbt, hook_dll.cbt_proc, &hook_dll.library);
        match mw.run_event_loop() {
            Ok(o) => {
                trace!("Exited successfully");
                o
            }
            Err(e) => {
                error!("Error running Windows loop: {}", e);
                1
            }
        }
    }());
}
