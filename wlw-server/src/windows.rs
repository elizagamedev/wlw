mod def;
mod wrappers;

pub use self::def::*;
pub use self::wrappers::*;

use std::ffi::{OsStr, OsString};
use std::os::windows::prelude::*;
use std::slice;
use std::{fmt, mem, ptr};

unsafe fn c_lpwstr_to_osstring(lpwstr: LPWSTR) -> OsString {
    let len = (0..).take_while(|&i| *lpwstr.offset(i) != 0).count();
    lpwstr_to_osstring(lpwstr, len)
}

unsafe fn lpwstr_to_osstring(lpwstr: LPWSTR, size: usize) -> OsString {
    let slice = slice::from_raw_parts(lpwstr as *const u16, size);
    OsString::from_wide(slice)
}

fn osstring_to_wstr<S: AsRef<OsStr>>(string: S) -> Vec<u16> {
    string
        .as_ref()
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect::<Vec<_>>()
}

fn format_error(code: DWORD, f: &mut fmt::Formatter) -> fmt::Result {
    unsafe {
        let mut buf: LPWSTR = mem::uninitialized();
        #[allow(clippy::crosspointer_transmute)]
        let buf_ptr = mem::transmute::<*mut LPWSTR, LPWSTR>(&mut buf as *mut LPWSTR);
        let size = FormatMessageW(
            FORMAT_MESSAGE_ALLOCATE_BUFFER
                | FORMAT_MESSAGE_FROM_SYSTEM
                | FORMAT_MESSAGE_IGNORE_INSERTS,
            ptr::null_mut(),
            code,
            DWORD::from(MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT)),
            buf_ptr,
            0,
            ptr::null_mut(),
        );
        if size == 0 {
            write!(f, "Unknown Windows error")
        } else {
            let result = lpwstr_to_osstring(buf, size as usize);
            LocalFree(buf as HLOCAL);

            // Format result
            write!(f, "{}", result.to_str().unwrap().trim_end())
        }
    }
}
