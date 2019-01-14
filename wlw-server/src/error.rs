use crate::util;
use std::error::Error;
use std::{fmt, mem, ptr};
use winapi::shared::minwindef::{DWORD, HLOCAL};
use winapi::shared::ntdef::{LANG_NEUTRAL, LPWSTR, MAKELANGID, SUBLANG_DEFAULT};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winbase::{
    FormatMessageW, LocalFree, FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_FROM_SYSTEM,
    FORMAT_MESSAGE_IGNORE_INSERTS,
};

#[derive(Debug)]
pub struct WindowsError(DWORD);

impl WindowsError {
    pub fn new(code: DWORD) -> WindowsError {
        if code == 0 {
            panic!("Created a WindowsError with success code");
        }
        WindowsError(code)
    }

    pub fn last() -> WindowsError {
        WindowsError::new(unsafe { GetLastError() })
    }
}

impl fmt::Display for WindowsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            let mut buf: LPWSTR = mem::uninitialized();
            #[allow(clippy::crosspointer_transmute)]
            let buf_ptr = mem::transmute::<*mut LPWSTR, LPWSTR>(&mut buf as *mut LPWSTR);
            let size = FormatMessageW(
                FORMAT_MESSAGE_ALLOCATE_BUFFER
                    | FORMAT_MESSAGE_FROM_SYSTEM
                    | FORMAT_MESSAGE_IGNORE_INSERTS,
                ptr::null_mut(),
                self.0,
                DWORD::from(MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT)),
                buf_ptr,
                0,
                ptr::null_mut(),
            );
            if size == 0 {
                panic!("Error while attempting to format error code {}", self.0);
            }
            let result = util::lpwstr_to_string(buf, size as usize)
                .expect("Error while attempting to decode utf-16 in error message");
            LocalFree(buf as HLOCAL);

            // Format result
            write!(f, "{}", result.trim_end())
        }
    }
}

impl Error for WindowsError {}

#[cfg(test)]
mod tests {
    use super::*;
    use winapi::shared::winerror::ERROR_FILE_NOT_FOUND;
    #[test]
    fn display() {
        assert_eq!(
            WindowsError::new(ERROR_FILE_NOT_FOUND).to_string(),
            "The system cannot find the file specified."
        );
    }

}
