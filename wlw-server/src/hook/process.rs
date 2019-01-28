use wlw_server::windows;

pub struct Process {
    handle: windows::HANDLE,
}

impl Process {
    pub fn new(pid: u32) -> windows::Result<Self> {
        let handle = unsafe { windows::OpenProcess(windows::SYNCHRONIZE, false, pid) }?;
        Ok(Process { handle })
    }

    pub fn try_wait(&self) -> windows::Result<Option<u32>> {
        match unsafe { windows::WaitForSingleObject(self.handle, 0) }? {
            windows::WaitResult::Timeout => Ok(None),
            windows::WaitResult::Object(_) => {
                Ok(Some(unsafe { windows::GetExitCodeProcess(self.handle) }?))
            }
            _ => unreachable!(),
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe { windows::CloseHandle(self.handle) }.unwrap();
    }
}
