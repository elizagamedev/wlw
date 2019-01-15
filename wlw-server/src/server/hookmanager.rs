#[cfg(debug_assertions)]
use crate::debug;
use std::path::PathBuf;
use std::process::{self, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const NUM_ARCHS: usize = 2;

struct HookBinaryPaths {
    dll_paths: [PathBuf; NUM_ARCHS],
    exe_paths: [PathBuf; NUM_ARCHS],
}

impl HookBinaryPaths {
    #[cfg(debug_assertions)]
    fn get() -> Self {
        let paths = debug::HookBinaryPaths::get();
        HookBinaryPaths {
            dll_paths: [paths.dll32, paths.dll64],
            exe_paths: [paths.exe32, paths.exe64],
        }
    }
}

pub struct HookManager {
    monitor_thread: Option<thread::JoinHandle<()>>,
    monitor_thread_run: Arc<AtomicBool>,
}

impl HookManager {
    pub fn new() -> Self {
        let mut processes: [Option<process::Child>; NUM_ARCHS] = Default::default();
        let monitor_thread_run = Arc::new(AtomicBool::new(true));

        let binary_paths = HookBinaryPaths::get();
        let pid = process::id();
        let start_process = move |arch: usize| match Command::new(&binary_paths.exe_paths[arch])
            .env("WLW_PID", pid.to_string())
            .env("WLW_HOOK_DLL", &binary_paths.dll_paths[arch])
            .spawn()
        {
            Ok(child) => {
                info!(
                    "Started hook process \"{}\"",
                    binary_paths.exe_paths[arch].to_str().unwrap()
                );
                Some(child)
            }
            Err(e) => {
                error!("Could not start hook process: {}", e);
                None
            }
        };

        let run_thread = monitor_thread_run.clone();
        let monitor_thread = Some(thread::spawn(move || {
            while run_thread.load(Ordering::Relaxed) {
                for (arch, process) in processes.iter_mut().enumerate() {
                    match process {
                        Some(child) => {
                            match child.try_wait() {
                                Ok(Some(status)) => {
                                    error!(
                                        "Hook process exited prematurely with status {}",
                                        status
                                    );
                                    *process = start_process(arch);
                                }
                                Ok(None) => {} // all is ok
                                Err(e) => panic!("Error waiting for hook process: {}", e),
                            }
                        }
                        None => {
                            // todo start process
                            *process = start_process(arch);
                        }
                    }
                }
                thread::sleep(Duration::from_millis(500));
            }
            // kill processes
            // TODO graceful kill
            for process in processes.iter_mut() {
                if let Some(child) = process {
                    if let Ok(None) = child.try_wait() {
                        child.kill().unwrap();
                    }
                }
            }
        }));
        HookManager {
            monitor_thread,
            monitor_thread_run,
        }
    }
}

impl Drop for HookManager {
    fn drop(&mut self) {
        self.monitor_thread_run.store(false, Ordering::Relaxed);
        if let Some(h) = self.monitor_thread.take() {
            h.join().unwrap();
        }
    }
}
