use std::slice;
use std::string::FromUtf16Error;
use winapi::shared::ntdef::LPWSTR;

pub unsafe fn c_lpwstr_to_string(lpwstr: LPWSTR) -> Result<String, FromUtf16Error> {
    // Find size of string, then call lpwstr_to_string
    let mut size: usize = 0;
    while *lpwstr.offset(size as isize) != 0 {
        size += 1;
    }
    lpwstr_to_string(lpwstr, size)
}

pub unsafe fn lpwstr_to_string(lpwstr: LPWSTR, size: usize) -> Result<String, FromUtf16Error> {
    let lpwstr_slice = slice::from_raw_parts(lpwstr as *const u16, size);
    String::from_utf16(lpwstr_slice)
}
