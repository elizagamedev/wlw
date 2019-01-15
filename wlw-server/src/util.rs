use std::ffi::{OsStr, OsString};
use std::os::windows::prelude::*;
use std::slice;
use winapi::shared::ntdef::LPWSTR;

pub unsafe fn c_lpwstr_to_osstring(lpwstr: LPWSTR) -> OsString {
    let len = (0..).take_while(|&i| *lpwstr.offset(i) != 0).count();
    lpwstr_to_osstring(lpwstr, len)
}

pub unsafe fn lpwstr_to_osstring(lpwstr: LPWSTR, size: usize) -> OsString {
    let slice = slice::from_raw_parts(lpwstr as *const u16, size);
    OsString::from_wide(slice)
}

pub fn osstring_to_wstr<S: AsRef<OsStr>>(string: S) -> Vec<u16> {
    string
        .as_ref()
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect::<Vec<_>>()
}
