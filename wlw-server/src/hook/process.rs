use std::mem;
use winapi::shared::minwindef::{DWORD, FALSE};
use winapi::shared::ntdef::HANDLE;
use winapi::shared::winerror::WAIT_TIMEOUT;
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::{GetExitCodeProcess, OpenProcess};
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::WAIT_OBJECT_0;
use winapi::um::winnt::SYNCHRONIZE;
use wlw_server::windowserror::WindowsError;

pub struct Process {
    handle: HANDLE,
}

impl Process {
    pub fn new(pid: u32) -> Result<Self, WindowsError> {
        let handle = unsafe { OpenProcess(SYNCHRONIZE, FALSE, pid) };
        if handle.is_null() {
            Err(WindowsError::last())
        } else {
            Ok(Process { handle })
        }
    }

    pub fn try_wait(&self) -> Result<Option<u32>, WindowsError> {
        match unsafe { WaitForSingleObject(self.handle, 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut exit_status: DWORD = unsafe { mem::uninitialized() };
                if unsafe { GetExitCodeProcess(self.handle, &mut exit_status as *mut DWORD) } == 0 {
                    Err(WindowsError::last())
                } else {
                    Ok(Some(exit_status))
                }
            }
            _ => Err(WindowsError::last()),
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}
