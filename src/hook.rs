use std::fmt;
use winapi::ctypes::*;
use winapi::shared::minwindef::{DWORD, HINSTANCE, LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::HHOOK;
use winapi::um::errhandlingapi::GetLastError;
pub use winapi::um::winuser::HOOKPROC;
use winapi::um::winuser::{SetWindowsHookExW, UnhookWindowsHookEx, WH_CALLWNDPROC, WH_CBT};

enum HookId {
    CallWndProc,
    Cbt,
}

impl HookId {
    fn get_winapi_int(&self) -> c_int {
        match self {
            CallWndProc => WH_CALLWNDPROC,
            Cbt => WH_CBT,
        }
    }
}

struct WindowsHook {
    hook: HHOOK,
}

impl WindowsHook {
    pub fn new(
        hook_id: HookId,
        hook_proc: HOOKPROC,
        instance: &HInstance,
    ) -> Result<Self, WindowsError> {
        let winapi_hook_id = hook_id.get_winapi_int();
        unsafe {
            let hook = SetWindowsHookExW(winapi_hook_id, hook_proc, instance.handle, 0);
            if hook == std::ptr::null_mut() {
                Err(WindowsError::new(GetLastError()))
            } else {
                Ok(WindowsHook { hook })
            }
        }
    }
}

impl Drop for WindowsHook {
    fn drop(&mut self) {
        unsafe {
            UnhookWindowsHookEx(self.hook);
        }
    }
}
