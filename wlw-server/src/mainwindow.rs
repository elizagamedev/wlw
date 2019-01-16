use crate::util;
use crate::windowserror::WindowsError;
use std::ffi::OsString;
use std::panic::{self, RefUnwindSafe};
use std::sync::Arc;
use std::{mem, ptr};
#[cfg(target_pointer_width = "64")]
use winapi::shared::basetsd::LONG_PTR;
use winapi::shared::minwindef::{ATOM, HINSTANCE, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::ntdef::LPCWSTR;
use winapi::shared::windef::HWND;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, PostMessageW, PostQuitMessage, RegisterClassExW, SetWindowLongPtrW,
    TranslateMessage, UnregisterClassW, GWLP_USERDATA, MSG, WM_CLOSE, WNDCLASSEXW,
};

unsafe extern "system" fn window_proc_bootstrap(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CLOSE {
        trace!("Received close signal");
        PostQuitMessage(0);
        return 0;
    }
    let mw_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if mw_ptr == 0 {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    } else {
        let mw = &mut *(mw_ptr as *mut MainWindow);
        let result = match panic::catch_unwind(|| (mw.proc)(hwnd, msg, wparam, lparam)) {
            Ok(result) => result,
            Err(_) => {
                mw.wndproc_panic = true;
                true
            }
        };
        if result {
            0
        } else {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

pub struct MainWindow {
    _wndclass: WindowClass,
    window: Window,
    proc: Box<dyn Fn(HWND, UINT, WPARAM, LPARAM) -> bool + RefUnwindSafe>,
    wndproc_panic: bool,
}

impl MainWindow {
    pub fn new(
        name: impl AsRef<str>,
        proc: Box<dyn Fn(HWND, UINT, WPARAM, LPARAM) -> bool + RefUnwindSafe>,
    ) -> Result<Arc<Self>, WindowsError> {
        let instance = unsafe { GetModuleHandleW(ptr::null()) };

        // Register class
        let wndclass = WindowClass::new(name.as_ref(), instance)?;
        let window = Window::new(&wndclass, name.as_ref(), instance)?;
        let mw = Arc::new(MainWindow {
            _wndclass: wndclass,
            window,
            proc,
            wndproc_panic: false,
        });
        unsafe {
            // A bug in winapi causes silliness with LONG_PTR on x86.
            let mw_ptr = Arc::into_raw(mw);
            #[cfg(target_pointer_width = "32")]
            let ptr = mw_ptr as i32;
            #[cfg(target_pointer_width = "64")]
            let ptr = mw_ptr as LONG_PTR;
            SetWindowLongPtrW((*mw_ptr).window.hwnd, GWLP_USERDATA, ptr);
            Ok(Arc::from_raw(mw_ptr))
        }
    }

    pub fn run_event_loop(&self) -> Result<i32, WindowsError> {
        loop {
            let mut msg: MSG = unsafe { mem::uninitialized() };
            let ret = unsafe { GetMessageW(&mut msg as *mut MSG, ptr::null_mut(), 0, 0) };
            if ret > 0 {
                unsafe {
                    TranslateMessage(&mut msg as *mut MSG);
                    DispatchMessageW(&mut msg as *mut MSG);
                }
                if self.wndproc_panic {
                    panic!("Panicked in wndproc thread");
                }
            } else if ret < 0 {
                return Err(WindowsError::last());
            } else {
                trace!("Received quit message");
                return Ok(msg.wParam as i32);
            }
        }
    }

    pub fn close(&self) {
        unsafe {
            PostMessageW(self.window.hwnd, WM_CLOSE, 0, 0);
        }
    }
}

unsafe impl Sync for MainWindow {}

unsafe impl Send for MainWindow {}

struct WindowClass {
    wndclass: ATOM,
    instance: HINSTANCE,
}

impl WindowClass {
    fn new(name: impl AsRef<str>, instance: HINSTANCE) -> Result<WindowClass, WindowsError> {
        trace!("Registering new window class \"{}\"", name.as_ref());
        let wide_name = util::osstring_to_wstr(OsString::from(name.as_ref()));
        let mut opts = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(window_proc_bootstrap),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: ptr::null_mut(),
            hCursor: ptr::null_mut(),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null_mut(),
            lpszClassName: wide_name.as_ptr(),
            hIconSm: ptr::null_mut(),
        };
        let wndclass = unsafe { RegisterClassExW(&mut opts as *mut WNDCLASSEXW) };
        if wndclass == 0 {
            Err(WindowsError::last())
        } else {
            Ok(WindowClass { wndclass, instance })
        }
    }
}

impl Drop for WindowClass {
    fn drop(&mut self) {
        unsafe {
            UnregisterClassW(self.wndclass as *mut u16, self.instance);
        }
    }
}

struct Window {
    hwnd: HWND,
}

impl Window {
    fn new(
        wndclass: &WindowClass,
        name: impl AsRef<str>,
        instance: HINSTANCE,
    ) -> Result<Window, WindowsError> {
        trace!("Creating new window with title \"{}\"", name.as_ref());
        let wide_name = util::osstring_to_wstr(OsString::from(name.as_ref()));
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                wndclass.wndclass as LPCWSTR,
                wide_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null_mut(),
            );
            if hwnd.is_null() {
                Err(WindowsError::last())
            } else {
                Ok(Window { hwnd })
            }
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};

    #[test]
    fn create_close_and_drop() {
        let mw = MainWindow::new("test_window", Box::new(|_, _, _, _| false)).unwrap();
        let mw_close = Arc::clone(&mw);
        let handle = thread::spawn(move || {
            thread::sleep(time::Duration::from_millis(1000));
            mw_close.close();
        });
        let rc = mw.run_event_loop().unwrap();
        handle.join().unwrap();
        assert_eq!(rc, 0);
    }

    #[test]
    #[should_panic]
    fn panic_in_window_proc() {
        let mw = MainWindow::new(
            "test_window",
            Box::new(|_, _, _, _| panic!("Panic inside window proc")),
        )
        .unwrap();
        mw.run_event_loop().unwrap();
    }
}
