use std::ffi::CString;
use std::mem;
use wlw_server::windows;

#[derive(Debug)]
pub enum HookId {
    CallWndProc,
    Cbt,
}

pub struct WindowsHook {
    hook_id: HookId,
    hook: windows::HHOOK,
}

impl WindowsHook {
    pub fn new(hook_id: HookId, hook_proc: HookProc, library: &Library) -> windows::Result<Self> {
        trace!("Registering Windows hook: {:?}", hook_id);
        let winapi_hook_id = match hook_id {
            HookId::CallWndProc => windows::WH_CALLWNDPROC,
            HookId::Cbt => windows::WH_CBT,
        };
        let hook = unsafe {
            windows::SetWindowsHookEx(winapi_hook_id, Some(hook_proc), library.handle, 0)
        }?;
        Ok(WindowsHook { hook_id, hook })
    }
}

impl Drop for WindowsHook {
    fn drop(&mut self) {
        trace!("Unregistering Windows hook: {:?}", self.hook_id);
        unsafe { windows::UnhookWindowsHookEx(self.hook) }.unwrap();
    }
}

pub struct Library {
    handle: windows::HMODULE,
}

impl Library {
    pub fn new(path: &str) -> windows::Result<Self> {
        let handle = unsafe { windows::LoadLibrary(path) }?;
        Ok(Library { handle })
    }

    pub fn get_proc_address(&self, name: &str) -> windows::Result<windows::FARPROC> {
        let c_name = CString::new(name).expect("CString::new failed");
        let result = unsafe { windows::GetProcAddress(self.handle, c_name) }?;
        Ok(result)
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe { windows::FreeLibrary(self.handle) }.unwrap();
    }
}

pub type HookProc = unsafe extern "system" fn(
    code: windows::c_int,
    wParam: windows::WPARAM,
    lParam: windows::LPARAM,
) -> windows::LRESULT;

pub struct HookDll {
    pub library: Library,
    pub callwndproc_proc: HookProc,
    pub cbt_proc: HookProc,
}

impl HookDll {
    pub fn new(library: Library, server_pid: u32) -> Result<HookDll, windows::Error> {
        unsafe {
            let callwndproc_proc = mem::transmute::<windows::FARPROC, HookProc>(
                library.get_proc_address("callwndproc_proc")?,
            );
            let cbt_proc =
                mem::transmute::<windows::FARPROC, HookProc>(library.get_proc_address("cbt_proc")?);
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
