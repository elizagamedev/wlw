use std::{mem, ptr};
use winapi::shared::minwindef::{FALSE, WPARAM};
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{
    DispatchMessageW, GetMessageW, PostThreadMessageW, TranslateMessage, MSG, WM_QUIT,
};
use wlw_server::windowserror::WindowsError;

pub fn get_current_thread_id() -> u32 {
    unsafe { GetCurrentThreadId() }
}

pub fn post_quit_message(thread_id: u32, code: i32) -> Result<(), WindowsError> {
    let result = unsafe { PostThreadMessageW(thread_id, WM_QUIT, code as WPARAM, 0) };
    if result == FALSE {
        Err(WindowsError::last())
    } else {
        Ok(())
    }
}

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
