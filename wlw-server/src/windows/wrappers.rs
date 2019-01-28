#![allow(non_snake_case)]
#![allow(clippy::too_many_arguments)]
use super::def::*;
use std::ffi::{CStr, OsStr, OsString};
use std::os::windows::prelude::*;
use std::{error, fmt, mem, ptr};
pub use winapi::um::errhandlingapi::{GetLastError, SetLastError};
pub use winapi::um::processthreadsapi::GetCurrentThreadId;
pub use winapi::um::winuser::DispatchMessageW;

#[derive(Debug)]
pub enum ErrorOrigin {
    SetWindowText,
    GetWindowTextLength,
    GetWindowText,
    GetWindowLong,
    SetWindowLong,
    GetWindowRect,
    SetWindowPos,
    ReadFile,
    WriteFile,
    CloseHandle,
    GetOverlappedResult,
    ConnectNamedPipe,
    CreateNamedPipe,
    DisconnectNamedPipe,
    ResetEvent,
    CreateEvent,
    SetEvent,
    WaitForMultipleObjects,
    CreateFile,
    SetNamedPipeHandleState,
    SetWindowsHookEx,
    UnhookWindowsHookEx,
    LoadLibrary,
    FreeLibrary,
    GetProcAddress,
    WaitForSingleObject,
    OpenProcess,
    GetExitCodeProcess,
    PostThreadMessage,
    GetMessage,
}

#[derive(Debug)]
pub struct Error {
    pub origin: ErrorOrigin,
    pub code: DWORD,
}

impl Error {
    fn new(origin: ErrorOrigin, code: DWORD) -> Self {
        Error { origin, code }
    }

    fn last(origin: ErrorOrigin) -> Self {
        Error {
            origin,
            code: unsafe { GetLastError() },
        }
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}: ", self.origin)?;
        super::format_error(self.code, f)?;
        write!(f, " [{}]", self.code)?;
        Ok(())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub unsafe fn SetWindowText(hWnd: HWND, string: impl AsRef<OsStr>) -> Result<()> {
    let wide_string = super::osstring_to_wstr(string);
    let result = winapi::um::winuser::SetWindowTextW(hWnd, wide_string.as_ptr());
    if result == FALSE {
        Err(Error::last(ErrorOrigin::SetWindowText))
    } else {
        Ok(())
    }
}

pub unsafe fn GetWindowTextLength(hWnd: HWND) -> Result<c_int> {
    let result = winapi::um::winuser::GetWindowTextLengthW(hWnd);
    if result == 0 {
        // TODO differentiate between errors and 0 length text
        let last_error = GetLastError();
        if last_error == ERROR_SUCCESS {
            Ok(0)
        } else {
            Err(Error::last(ErrorOrigin::GetWindowTextLength))
        }
    } else {
        Ok(result)
    }
}

pub unsafe fn GetWindowText(hWnd: HWND) -> Result<OsString> {
    let title_length = GetWindowTextLength(hWnd)?;
    if title_length > 0 {
        let size = (title_length + 1) as usize;
        let mut title_buffer: Vec<u16> = vec![mem::uninitialized(); size];
        let ret = winapi::um::winuser::GetWindowTextW(hWnd, title_buffer.as_mut_ptr(), size as i32);
        if ret == 0 {
            Err(Error::last(ErrorOrigin::GetWindowText))
        } else {
            Ok(OsString::from_wide(&title_buffer[..ret as usize]))
        }
    } else {
        Ok(OsString::new())
    }
}

pub unsafe fn GetWindowLong(hWnd: HWND, nIndex: c_int) -> Result<LONG> {
    let result = winapi::um::winuser::GetWindowLongW(hWnd, nIndex);
    if result == 0 {
        let last_error = GetLastError();
        if last_error == ERROR_SUCCESS {
            Ok(0)
        } else {
            Err(Error::new(ErrorOrigin::GetWindowLong, last_error))
        }
    } else {
        Ok(result)
    }
}

pub unsafe fn SetWindowLong(hWnd: HWND, nIndex: c_int, dwNewLong: LONG) -> Result<LONG> {
    SetLastError(ERROR_SUCCESS);
    let result = winapi::um::winuser::SetWindowLongW(hWnd, nIndex, dwNewLong);
    if result == 0 {
        let last_error = GetLastError();
        if last_error == ERROR_SUCCESS {
            Ok(0)
        } else {
            Err(Error::new(ErrorOrigin::SetWindowLong, last_error))
        }
    } else {
        Ok(result)
    }
}

pub unsafe fn GetWindowRect(hWnd: HWND) -> Result<RECT> {
    let mut rect: RECT = mem::uninitialized();
    let result = winapi::um::winuser::GetWindowRect(hWnd, &mut rect as *mut _);
    if result == FALSE {
        Err(Error::last(ErrorOrigin::GetWindowRect))
    } else {
        Ok(rect)
    }
}

pub unsafe fn SetWindowPos(
    hWnd: HWND,
    hWndInsertAfter: HWND,
    X: c_int,
    Y: c_int,
    cx: c_int,
    cy: c_int,
    uFlags: UINT,
) -> Result<()> {
    let result = winapi::um::winuser::SetWindowPos(hWnd, hWndInsertAfter, X, Y, cx, cy, uFlags);
    if result == FALSE {
        Err(Error::last(ErrorOrigin::SetWindowPos))
    } else {
        Ok(())
    }
}

pub enum IoState {
    Pending,
    Finished,
}

pub unsafe fn ReadFile(
    hFile: HANDLE,
    lpBuffer: LPVOID,
    nNumberOfBytesToRead: DWORD,
    lpNumberOfBytesRead: LPDWORD,
    lpOverlapped: LPOVERLAPPED,
) -> Result<IoState> {
    let result = winapi::um::fileapi::ReadFile(
        hFile,
        lpBuffer,
        nNumberOfBytesToRead,
        lpNumberOfBytesRead,
        lpOverlapped,
    );
    if result == FALSE {
        let last_error = GetLastError();
        if last_error == ERROR_IO_PENDING {
            Ok(IoState::Pending)
        } else {
            Err(Error::new(ErrorOrigin::ReadFile, last_error))
        }
    } else {
        Ok(IoState::Finished)
    }
}

pub unsafe fn WriteFile(
    hFile: HANDLE,
    lpBuffer: LPCVOID,
    nNumberOfBytesToWrite: DWORD,
    lpNumberOfBytesWritten: LPDWORD,
    lpOverlapped: LPOVERLAPPED,
) -> Result<IoState> {
    let result = winapi::um::fileapi::WriteFile(
        hFile,
        lpBuffer,
        nNumberOfBytesToWrite,
        lpNumberOfBytesWritten,
        lpOverlapped,
    );
    if result == FALSE {
        let last_error = GetLastError();
        if last_error == ERROR_IO_PENDING {
            Ok(IoState::Pending)
        } else {
            Err(Error::new(ErrorOrigin::WriteFile, last_error))
        }
    } else {
        Ok(IoState::Finished)
    }
}

pub unsafe fn CloseHandle(hObject: HANDLE) -> Result<()> {
    let result = winapi::um::handleapi::CloseHandle(hObject);
    if result == FALSE {
        Err(Error::last(ErrorOrigin::CloseHandle))
    } else {
        Ok(())
    }
}

pub unsafe fn GetOverlappedResult(
    hFile: HANDLE,
    lpOverlapped: LPOVERLAPPED,
    bWait: bool,
) -> Result<DWORD> {
    let mut num_transferred: DWORD = mem::uninitialized();
    let success = winapi::um::ioapiset::GetOverlappedResult(
        hFile,
        lpOverlapped,
        &mut num_transferred as *mut DWORD,
        if bWait { TRUE } else { FALSE },
    );
    if success == FALSE {
        Err(Error::last(ErrorOrigin::GetOverlappedResult))
    } else {
        Ok(num_transferred)
    }
}

pub unsafe fn ConnectNamedPipe(hNamedPipe: HANDLE, lpOverlapped: LPOVERLAPPED) -> Result<IoState> {
    let result = winapi::um::namedpipeapi::ConnectNamedPipe(hNamedPipe, lpOverlapped);
    if result == FALSE {
        let last_error = GetLastError();
        match last_error {
            ERROR_PIPE_CONNECTED => Ok(IoState::Finished),
            ERROR_IO_PENDING => Ok(IoState::Pending),
            _ => Err(Error::new(ErrorOrigin::ConnectNamedPipe, last_error)),
        }
    } else {
        Ok(IoState::Finished)
    }
}

pub unsafe fn CreateNamedPipe(
    name: impl AsRef<OsStr>,
    dwOpenMode: DWORD,
    dwPipeMode: DWORD,
    nMaxInstances: DWORD,
    nOutBufferSize: DWORD,
    nInBufferSize: DWORD,
    nDefaultTimeOut: DWORD,
    lpSecurityAttributes: LPSECURITY_ATTRIBUTES,
) -> Result<HANDLE> {
    let wide_name = super::osstring_to_wstr(name);
    let result = winapi::um::namedpipeapi::CreateNamedPipeW(
        wide_name.as_ptr(),
        dwOpenMode,
        dwPipeMode,
        nMaxInstances,
        nOutBufferSize,
        nInBufferSize,
        nDefaultTimeOut,
        lpSecurityAttributes,
    );
    if result == INVALID_HANDLE_VALUE {
        Err(Error::last(ErrorOrigin::CreateNamedPipe))
    } else {
        Ok(result)
    }
}

pub unsafe fn DisconnectNamedPipe(hNamedPipe: HANDLE) -> Result<()> {
    let result = winapi::um::namedpipeapi::DisconnectNamedPipe(hNamedPipe);
    if result == FALSE {
        Err(Error::last(ErrorOrigin::DisconnectNamedPipe))
    } else {
        Ok(())
    }
}

pub unsafe fn ResetEvent(hEvent: HANDLE) -> Result<()> {
    let result = winapi::um::synchapi::ResetEvent(hEvent);
    if result == FALSE {
        Err(Error::last(ErrorOrigin::ResetEvent))
    } else {
        Ok(())
    }
}

pub unsafe fn CreateEvent(
    lpEventAttributes: LPSECURITY_ATTRIBUTES,
    bManualReset: bool,
    bInitialState: bool,
) -> Result<HANDLE> {
    let result = winapi::um::synchapi::CreateEventW(
        lpEventAttributes,
        if bManualReset { TRUE } else { FALSE },
        if bInitialState { TRUE } else { FALSE },
        ptr::null(),
    );
    if result.is_null() {
        Err(Error::last(ErrorOrigin::CreateEvent))
    } else {
        Ok(result)
    }
}

pub unsafe fn SetEvent(hEvent: HANDLE) -> Result<()> {
    let result = winapi::um::synchapi::SetEvent(hEvent);
    if result == FALSE {
        Err(Error::last(ErrorOrigin::SetEvent))
    } else {
        Ok(())
    }
}

pub enum WaitResult {
    Object(DWORD),
    Abandoned(DWORD),
    Timeout,
}

#[allow(clippy::absurd_extreme_comparisons)]
pub unsafe fn WaitForMultipleObjects(
    nCount: DWORD,
    lpHandles: *const HANDLE,
    bWaitAll: bool,
    dwMilliseconds: DWORD,
) -> Result<WaitResult> {
    let result = winapi::um::synchapi::WaitForMultipleObjects(
        nCount,
        lpHandles,
        if bWaitAll { TRUE } else { FALSE },
        dwMilliseconds,
    );
    if result == WAIT_FAILED {
        Err(Error::last(ErrorOrigin::WaitForMultipleObjects))
    } else if result == WAIT_TIMEOUT {
        Ok(WaitResult::Timeout)
    } else if result >= WAIT_OBJECT_0 && result < WAIT_OBJECT_0 + nCount {
        Ok(WaitResult::Object(result - WAIT_OBJECT_0))
    } else if result >= WAIT_ABANDONED_0 && result < WAIT_ABANDONED_0 + nCount {
        Ok(WaitResult::Abandoned(result - WAIT_ABANDONED_0))
    } else {
        unreachable!()
    }
}

pub unsafe fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) -> Result<WaitResult> {
    let result = winapi::um::synchapi::WaitForSingleObject(hHandle, dwMilliseconds);
    match result {
        WAIT_TIMEOUT => Ok(WaitResult::Timeout),
        WAIT_OBJECT_0 => Ok(WaitResult::Object(0)),
        WAIT_ABANDONED_0 => Ok(WaitResult::Abandoned(0)),
        WAIT_FAILED => Err(Error::last(ErrorOrigin::WaitForSingleObject)),
        _ => unreachable!(),
    }
}

pub unsafe fn OpenProcess(
    dwDesiredAccess: DWORD,
    bInheritHandle: bool,
    dwProcessId: DWORD,
) -> Result<HANDLE> {
    let result = winapi::um::processthreadsapi::OpenProcess(
        dwDesiredAccess,
        if bInheritHandle { TRUE } else { FALSE },
        dwProcessId,
    );
    if result.is_null() {
        Err(Error::last(ErrorOrigin::OpenProcess))
    } else {
        Ok(result)
    }
}

pub unsafe fn GetExitCodeProcess(hProcess: HANDLE) -> Result<DWORD> {
    let mut exit_status: DWORD = mem::uninitialized();
    let result =
        winapi::um::processthreadsapi::GetExitCodeProcess(hProcess, &mut exit_status as *mut _);
    if result == FALSE {
        Err(Error::last(ErrorOrigin::GetExitCodeProcess))
    } else {
        Ok(exit_status)
    }
}

pub unsafe fn CreateFile(
    file_name: impl AsRef<OsStr>,
    dwDesiredAccess: DWORD,
    dwShareMode: DWORD,
    lpSecurityAttributes: LPSECURITY_ATTRIBUTES,
    dwCreationDisposition: DWORD,
    dwFlagsAndAttributes: DWORD,
    hTemplateFile: HANDLE,
) -> Result<HANDLE> {
    let wide_name = super::osstring_to_wstr(file_name);
    let result = winapi::um::fileapi::CreateFileW(
        wide_name.as_ptr(),
        dwDesiredAccess,
        dwShareMode,
        lpSecurityAttributes,
        dwCreationDisposition,
        dwFlagsAndAttributes,
        hTemplateFile,
    );
    if result == INVALID_HANDLE_VALUE {
        Err(Error::last(ErrorOrigin::CreateFile))
    } else {
        Ok(result)
    }
}

pub unsafe fn SetNamedPipeHandleState(
    hNamedPipe: HANDLE,
    lpMode: LPDWORD,
    lpMaxCollectionCount: LPDWORD,
    lpCollectDataTimeout: LPDWORD,
) -> Result<()> {
    let result = winapi::um::namedpipeapi::SetNamedPipeHandleState(
        hNamedPipe,
        lpMode,
        lpMaxCollectionCount,
        lpCollectDataTimeout,
    );
    if result == FALSE {
        Err(Error::last(ErrorOrigin::SetNamedPipeHandleState))
    } else {
        Ok(())
    }
}

pub unsafe fn SetWindowsHookEx(
    idHook: c_int,
    lpfn: HOOKPROC,
    hmod: HINSTANCE,
    dwThreadId: DWORD,
) -> Result<HHOOK> {
    let result = winapi::um::winuser::SetWindowsHookExW(idHook, lpfn, hmod, dwThreadId);
    if result.is_null() {
        Err(Error::last(ErrorOrigin::SetWindowsHookEx))
    } else {
        Ok(result)
    }
}

pub unsafe fn UnhookWindowsHookEx(hhk: HHOOK) -> Result<()> {
    let result = winapi::um::winuser::UnhookWindowsHookEx(hhk);
    if result == FALSE {
        Err(Error::last(ErrorOrigin::UnhookWindowsHookEx))
    } else {
        Ok(())
    }
}

pub unsafe fn LoadLibrary(file_name: impl AsRef<OsStr>) -> Result<HMODULE> {
    let wide_name = super::osstring_to_wstr(file_name);
    let handle = winapi::um::libloaderapi::LoadLibraryW(wide_name.as_ptr());
    if handle.is_null() {
        Err(Error::last(ErrorOrigin::LoadLibrary))
    } else {
        Ok(handle)
    }
}

pub unsafe fn FreeLibrary(hLibModule: HMODULE) -> Result<()> {
    let result = winapi::um::libloaderapi::FreeLibrary(hLibModule);
    if result == FALSE {
        Err(Error::last(ErrorOrigin::FreeLibrary))
    } else {
        Ok(())
    }
}

pub unsafe fn GetProcAddress(hModule: HMODULE, name: impl AsRef<CStr>) -> Result<FARPROC> {
    let result = winapi::um::libloaderapi::GetProcAddress(hModule, name.as_ref().as_ptr());
    if result.is_null() {
        Err(Error::last(ErrorOrigin::GetProcAddress))
    } else {
        Ok(result)
    }
}

pub unsafe fn PostThreadMessage(
    idThread: DWORD,
    msg: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
) -> Result<()> {
    let result = winapi::um::winuser::PostThreadMessageW(idThread, msg, wParam, lParam);
    if result == FALSE {
        Err(Error::last(ErrorOrigin::PostThreadMessage))
    } else {
        Ok(())
    }
}

pub enum GetMessageResult {
    Quit(i32),
    Message(MSG),
}

pub unsafe fn GetMessage(
    hWnd: HWND,
    wMsgFilterMin: UINT,
    wMsgFilterMax: UINT,
) -> Result<GetMessageResult> {
    let mut msg: MSG = mem::uninitialized();
    let result =
        winapi::um::winuser::GetMessageW(&mut msg as *mut _, hWnd, wMsgFilterMin, wMsgFilterMax);
    if result == -1 {
        Err(Error::last(ErrorOrigin::GetMessage))
    } else if result == 0 {
        Ok(GetMessageResult::Quit(msg.wParam as i32))
    } else {
        Ok(GetMessageResult::Message(msg))
    }
}

pub unsafe fn TranslateMessage(lpmsg: *const MSG) -> bool {
    winapi::um::winuser::TranslateMessage(lpmsg) != FALSE
}
