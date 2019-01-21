use rlua;
use rlua::ToLua;
use std::error;
use std::ffi::OsString;
use std::fmt;
use std::mem;
use std::os::windows::prelude::*;
use std::sync::Arc;
use winapi::shared::minwindef::FALSE;
use winapi::shared::windef::HWND;
use winapi::shared::windef::RECT;
use winapi::um::winuser::GetWindowLongW;
use winapi::um::winuser::GetWindowRect;
use winapi::um::winuser::GetWindowTextLengthW;
use winapi::um::winuser::GetWindowTextW;
use winapi::um::winuser::SetWindowPos;
use winapi::um::winuser::SetWindowTextW;
use winapi::um::winuser::GWL_EXSTYLE;
use winapi::um::winuser::GWL_STYLE;
use winapi::um::winuser::HWND_TOP;
use winapi::um::winuser::SWP_NOACTIVATE;
use winapi::um::winuser::{
    SW_FORCEMINIMIZE, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, SW_SHOWDEFAULT,
    SW_SHOWMINIMIZED, SW_SHOWMINNOACTIVE, SW_SHOWNA, SW_SHOWNOACTIVATE, SW_SHOWNORMAL,
};
use winapi::um::winuser::{
    WS_BORDER, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_DISABLED, WS_DLGFRAME,
    WS_GROUP, WS_HSCROLL, WS_ICONIC, WS_MAXIMIZE, WS_MAXIMIZEBOX, WS_MINIMIZE, WS_MINIMIZEBOX,
    WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_VISIBLE, WS_VSCROLL,
};
use winapi::um::winuser::{
    WS_EX_ACCEPTFILES, WS_EX_APPWINDOW, WS_EX_CLIENTEDGE, WS_EX_COMPOSITED, WS_EX_CONTEXTHELP,
    WS_EX_CONTROLPARENT, WS_EX_DLGMODALFRAME, WS_EX_LAYERED, WS_EX_LAYOUTRTL, WS_EX_LEFTSCROLLBAR,
    WS_EX_MDICHILD, WS_EX_NOACTIVATE, WS_EX_NOINHERITLAYOUT, WS_EX_NOPARENTNOTIFY,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_RIGHT, WS_EX_RTLREADING, WS_EX_STATICEDGE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_EX_WINDOWEDGE,
};

use wlw_server::util;

#[derive(Debug)]
enum Error {
    KeyDoesNotExist(String),
    WindowIsDestroyed,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::KeyDoesNotExist(key) => write!(f, "Key does not exist: {}", key),
            Error::WindowIsDestroyed => write!(f, "Attempted to access value on destroyed window"),
        }
    }
}

pub struct WindowHandle {
    hwnd: HWND,
    is_destroyed: bool,
}

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}

impl WindowHandle {
    pub fn new(hwnd: HWND) -> Self {
        WindowHandle {
            hwnd,
            is_destroyed: false,
        }
    }

    fn get_title(&self) -> String {
        assert!(!self.is_destroyed);
        let title_length = unsafe { GetWindowTextLengthW(self.hwnd) };
        if title_length > 0 {
            let size = (title_length + 1) as usize;
            let mut title_buffer: Vec<u16> = vec![unsafe { mem::uninitialized() }; size];
            let ret = unsafe { GetWindowTextW(self.hwnd, title_buffer.as_mut_ptr(), size as i32) };
            if ret == 0 {
                String::new()
            } else {
                OsString::from_wide(&title_buffer[..ret as usize])
                    .into_string()
                    .unwrap()
            }
        } else {
            String::new()
        }
    }

    fn set_title(&self, title: &str) {
        assert!(!self.is_destroyed);
        let title_buffer = util::osstring_to_wstr(OsString::from(title));
        unsafe { SetWindowTextW(self.hwnd, title_buffer.as_ptr()) };
    }

    fn get_window_rect(&self) -> Rect {
        unsafe {
            let mut rect: RECT = mem::uninitialized();
            if GetWindowRect(self.hwnd, &mut rect as *mut RECT) == FALSE {
                Rect {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                }
            } else {
                Rect::from(rect)
            }
        }
    }

    fn set_window_rect(&self, x: i32, y: i32, w: i32, h: i32) {
        unsafe { SetWindowPos(self.hwnd, HWND_TOP, x, y, w, h, SWP_NOACTIVATE) };
    }
}

impl rlua::UserData for WindowHandle {
    fn add_methods<'lua, M: rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_window_rect", |_, this, ()| Ok(this.get_window_rect()));

        methods.add_method("set_window_rect", |_, this, args: (i32, i32, i32, i32)| {
            Ok(this.set_window_rect(args.0, args.1, args.2, args.3))
        });

        methods.add_meta_method(
            rlua::MetaMethod::Index,
            |lua_ctx, this, key: String| -> rlua::Result<rlua::Value> {
                if this.is_destroyed {
                    Err(rlua::Error::ExternalError(Arc::new(
                        Error::WindowIsDestroyed,
                    )))
                } else {
                    match key.as_ref() {
                        "title" => this.get_title().to_lua(lua_ctx),
                        "style" => WindowStyle::new(this.hwnd).to_lua(lua_ctx),
                        _ => Err(rlua::Error::ExternalError(Arc::new(
                            Error::KeyDoesNotExist(key),
                        ))),
                    }
                }
            },
        );
    }
}

struct WindowStyle {
    style: u32,
    ex_style: u32,
}

impl WindowStyle {
    fn new(hwnd: HWND) -> Self {
        unsafe {
            WindowStyle {
                style: GetWindowLongW(hwnd, GWL_STYLE) as u32,
                ex_style: GetWindowLongW(hwnd, GWL_EXSTYLE) as u32,
            }
        }
    }

    fn str_to_style_flags(key: &str) -> Option<u32> {
        match key {
            "border" => Some(WS_BORDER),
            "caption" => Some(WS_CAPTION),
            "child" => Some(WS_CHILD),
            "clipchildren" => Some(WS_CLIPCHILDREN),
            "clipsiblings" => Some(WS_CLIPSIBLINGS),
            "disabled" => Some(WS_DISABLED),
            "dlgframe" => Some(WS_DLGFRAME),
            "group" => Some(WS_GROUP),
            "hscroll" => Some(WS_HSCROLL),
            "iconic" => Some(WS_ICONIC),
            "maximize" => Some(WS_MAXIMIZE),
            "maximizebox" => Some(WS_MAXIMIZEBOX),
            "minimize" => Some(WS_MINIMIZE),
            "minimizebox" => Some(WS_MINIMIZEBOX),
            "popup" => Some(WS_POPUP),
            "sysmenu" => Some(WS_SYSMENU),
            "tabstop" => Some(WS_TABSTOP),
            "thickframe" => Some(WS_THICKFRAME),
            "visible" => Some(WS_VISIBLE),
            "vscroll" => Some(WS_VSCROLL),
            _ => None,
        }
    }

    fn str_to_ex_style_flags(key: &str) -> Option<u32> {
        match key {
            "acceptfiles" => Some(WS_EX_ACCEPTFILES),
            "appwindow" => Some(WS_EX_APPWINDOW),
            "clientedge" => Some(WS_EX_CLIENTEDGE),
            "composited" => Some(WS_EX_COMPOSITED),
            "contexthelp" => Some(WS_EX_CONTEXTHELP),
            "controlparent" => Some(WS_EX_CONTROLPARENT),
            "dlgmodalframe" => Some(WS_EX_DLGMODALFRAME),
            "layered" => Some(WS_EX_LAYERED),
            "layoutrtl" => Some(WS_EX_LAYOUTRTL),
            "leftscrollbar" => Some(WS_EX_LEFTSCROLLBAR),
            "mdichild" => Some(WS_EX_MDICHILD),
            "noactivate" => Some(WS_EX_NOACTIVATE),
            "noinheritlayout" => Some(WS_EX_NOINHERITLAYOUT),
            "noparentnotify" => Some(WS_EX_NOPARENTNOTIFY),
            "noredirectionbitmap" => Some(WS_EX_NOREDIRECTIONBITMAP),
            "right" => Some(WS_EX_RIGHT),
            "rtlreading" => Some(WS_EX_RTLREADING),
            "staticedge" => Some(WS_EX_STATICEDGE),
            "toolwindow" => Some(WS_EX_TOOLWINDOW),
            "topmost" => Some(WS_EX_TOPMOST),
            "transparent" => Some(WS_EX_TRANSPARENT),
            "windowedge" => Some(WS_EX_WINDOWEDGE),
            _ => None,
        }
    }
}

impl rlua::UserData for WindowStyle {
    fn add_methods<'lua, M: rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(rlua::MetaMethod::Index, |_, this, key: String| {
            match WindowStyle::str_to_style_flags(key.as_ref()) {
                Some(flags) => Ok(this.style & flags != 0),
                None => match WindowStyle::str_to_ex_style_flags(key.as_ref()) {
                    Some(flags) => Ok(this.ex_style & flags != 0),
                    None => Err(rlua::Error::ExternalError(Arc::new(
                        Error::KeyDoesNotExist(key),
                    ))),
                },
            }
        });
    }
}

#[derive(Copy, Clone)]
pub struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl From<RECT> for Rect {
    fn from(rect: RECT) -> Self {
        Rect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        }
    }
}

impl From<Rect> for RECT {
    fn from(rect: Rect) -> Self {
        RECT {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        }
    }
}

impl rlua::UserData for Rect {
    fn add_methods<'lua, M: rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(rlua::MetaMethod::Index, |_, this, key: String| {
            match key.as_ref() {
                "left" => Ok(this.left),
                "top" => Ok(this.top),
                "right" => Ok(this.right),
                "bottom" => Ok(this.bottom),
                "x" => Ok(this.left),
                "y" => Ok(this.top),
                "width" => Ok(this.right - this.left),
                "height" => Ok(this.bottom - this.top),
                _ => Err(rlua::Error::ExternalError(Arc::new(
                    Error::KeyDoesNotExist(key),
                ))),
            }
        });

        methods.add_meta_method_mut(
            rlua::MetaMethod::NewIndex,
            |_, this, args: (String, i32)| {
                let key = args.0;
                let val = args.1;
                match key.as_ref() {
                    "left" => {
                        this.left = val;
                        Ok(val)
                    }
                    "top" => {
                        this.top = val;
                        Ok(val)
                    }
                    "right" => {
                        this.right = val;
                        Ok(val)
                    }
                    "bottom" => {
                        this.bottom = val;
                        Ok(val)
                    }
                    "x" => {
                        this.left = val;
                        Ok(val)
                    }
                    "y" => {
                        this.top = val;
                        Ok(val)
                    }
                    "width" => {
                        this.right = this.left + val;
                        Ok(val)
                    }
                    "height" => {
                        this.bottom = this.top + val;
                        Ok(val)
                    }
                    _ => Err(rlua::Error::ExternalError(Arc::new(
                        Error::KeyDoesNotExist(key),
                    ))),
                }
            },
        );
    }
}

pub fn show_command_to_str(cmd: i32) -> Option<&'static str> {
    match cmd {
        SW_FORCEMINIMIZE => Some("forceminimize"),
        SW_HIDE => Some("hide"),
        SW_MAXIMIZE => Some("maximize"),
        SW_MINIMIZE => Some("minimize"),
        SW_RESTORE => Some("restore"),
        SW_SHOW => Some("show"),
        SW_SHOWDEFAULT => Some("showdefault"),
        SW_SHOWMINIMIZED => Some("showminimized"),
        SW_SHOWMINNOACTIVE => Some("showminnoactive"),
        SW_SHOWNA => Some("showna"),
        SW_SHOWNOACTIVATE => Some("shownoactivate"),
        SW_SHOWNORMAL => Some("shownormal"),
        _ => None,
    }
}
