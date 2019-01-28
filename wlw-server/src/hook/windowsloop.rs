use std::ptr;
use wlw_server::windows;

pub fn post_quit_message(thread_id: u32, code: i32) -> windows::Result<()> {
    unsafe { windows::PostThreadMessage(thread_id, windows::WM_QUIT, code as windows::WPARAM, 0) }
}

pub fn run_event_loop() -> windows::Result<i32> {
    loop {
        match unsafe { windows::GetMessage(ptr::null_mut(), 0, 0) }? {
            windows::GetMessageResult::Quit(code) => return Ok(code),
            windows::GetMessageResult::Message(mut msg) => unsafe {
                windows::TranslateMessage(&mut msg as *mut _);
                windows::DispatchMessageW(&mut msg as *mut _);
            },
        }
    }
}
