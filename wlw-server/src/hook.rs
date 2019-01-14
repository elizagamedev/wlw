use crate::error::WindowsError;
use std::ffi::CString;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use winapi::ctypes::c_int;
use winapi::shared::minwindef::{FARPROC, HMODULE, LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::HHOOK;
use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryW};
pub use winapi::um::winuser::HOOKPROC;
use winapi::um::winuser::{SetWindowsHookExW, UnhookWindowsHookEx, WH_CALLWNDPROC, WH_CBT};

#[derive(Debug)]
pub enum HookId {
    CallWndProc,
    Cbt,
}

pub struct WindowsHook {
    hook_id: HookId,
    hook: HHOOK,
}

impl WindowsHook {
    pub fn new(
        hook_id: HookId,
        hook_proc: HookProc,
        library: &Library,
    ) -> Result<Self, WindowsError> {
        trace!("Registering Windows hook: {:?}", hook_id);
        let winapi_hook_id = match hook_id {
            HookId::CallWndProc => WH_CALLWNDPROC,
            HookId::Cbt => WH_CBT,
        };
        unsafe {
            let hook = SetWindowsHookExW(winapi_hook_id, Some(hook_proc), library.handle, 0);
            if hook.is_null() {
                Err(WindowsError::last())
            } else {
                Ok(WindowsHook { hook_id, hook })
            }
        }
    }
}

impl Drop for WindowsHook {
    fn drop(&mut self) {
        trace!("Unregistering Windows hook: {:?}", self.hook_id);
        unsafe {
            UnhookWindowsHookEx(self.hook);
        }
    }
}

pub struct Library {
    handle: HMODULE,
}

impl Library {
    pub fn new(path: &Path) -> Result<Self, WindowsError> {
        let wide_path = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect::<Vec<_>>();
        let handle = unsafe { LoadLibraryW(wide_path.as_ptr()) };
        if handle.is_null() {
            Err(WindowsError::last())
        } else {
            Ok(Library { handle })
        }
    }

    pub fn get_proc_address(&self, name: &str) -> Result<FARPROC, WindowsError> {
        let c_name = CString::new(name).expect("CString::new failed");
        let result = unsafe { GetProcAddress(self.handle, c_name.as_ptr()) };
        if result.is_null() {
            Err(WindowsError::last())
        } else {
            Ok(result)
        }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe { FreeLibrary(self.handle) };
    }
}

pub type HookProc =
    unsafe extern "system" fn(code: c_int, wParam: WPARAM, lParam: LPARAM) -> LRESULT;

pub struct HookDll {
    pub library: Library,
    pub callwndproc_proc: HookProc,
    pub cbt_proc: HookProc,
}

impl HookDll {
    pub fn new(library: Library, server_pid: u32) -> Result<HookDll, WindowsError> {
        unsafe {
            let callwndproc_proc =
                mem::transmute::<FARPROC, HookProc>(library.get_proc_address("callwndproc_proc")?);
            let cbt_proc =
                mem::transmute::<FARPROC, HookProc>(library.get_proc_address("cbt_proc")?);
            #[allow(clippy::cast_ptr_alignment)]
            let server_pid_ptr = library.get_proc_address("server_pid")? as *mut u32;
            *server_pid_ptr = server_pid;
            Ok(HookDll {
                library,
                callwndproc_proc,
                cbt_proc,
            })
        }
    }
}
