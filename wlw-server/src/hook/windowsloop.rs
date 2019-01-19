use std::{mem, ptr};
use winapi::um::winuser::{DispatchMessageW, GetMessageW, TranslateMessage, MSG};
use wlw_server::windowserror::WindowsError;

pub fn run_event_loop() -> Result<i32, WindowsError> {
    loop {
        let mut msg: MSG = unsafe { mem::uninitialized() };
        let ret = unsafe { GetMessageW(&mut msg as *mut MSG, ptr::null_mut(), 0, 0) };
        if ret > 0 {
            unsafe {
                TranslateMessage(&mut msg as *mut MSG);
                DispatchMessageW(&mut msg as *mut MSG);
            }
        } else if ret < 0 {
            return Err(WindowsError::last());
        } else {
            return Ok(msg.wParam as i32);
        }
    }
}
