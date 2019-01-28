use rlua;
use rlua::ToLua;
use std::error;
use std::ffi::OsString;
use std::fmt;
use std::sync::Arc;
use wlw_server::windows;

#[derive(Debug)]
enum Error {
    KeyDoesNotExist(String),
    WindowsError(windows::Error),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::KeyDoesNotExist(key) => write!(f, "Key does not exist: {}", key),
            Error::WindowsError(e) => write!(f, "Windows error: {}", e),
        }
    }
}

impl From<Error> for rlua::Error {
    fn from(err: Error) -> Self {
        rlua::Error::ExternalError(Arc::new(err))
    }
}

impl From<windows::Error> for Error {
    fn from(err: windows::Error) -> Self {
        Error::WindowsError(err)
    }
}

type Result<T> = std::result::Result<T, Error>;

pub struct WindowHandle {
    hwnd: windows::HWND,
}

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}

impl WindowHandle {
    pub fn new(hwnd: windows::HWND) -> Self {
        WindowHandle { hwnd }
    }

    fn get_title(&self) -> Result<String> {
        Ok(unsafe { windows::GetWindowText(self.hwnd) }
            .map(|s| s.into_string().unwrap_or_default())?)
    }

    fn set_title(&self, title: impl AsRef<str>) -> Result<()> {
        unsafe { windows::SetWindowText(self.hwnd, OsString::from(title.as_ref())) }?;
        Ok(())
    }

    fn get_window_rect(&self) -> Result<Rect> {
        Ok(unsafe { windows::GetWindowRect(self.hwnd) }.map(Rect::from)?)
    }

    fn set_window_rect(&self, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
        unsafe {
            windows::SetWindowPos(
                self.hwnd,
                windows::HWND_TOP,
                x,
                y,
                w,
                h,
                windows::SWP_NOACTIVATE,
            )
        }
        .map_err(|e| e.into())
    }
}

impl rlua::UserData for WindowHandle {
    fn add_methods<'lua, M: rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_window_rect", |_, this, ()| Ok(this.get_window_rect()?));

        methods.add_method("set_window_rect", |_, this, args: (i32, i32, i32, i32)| {
            this.set_window_rect(args.0, args.1, args.2, args.3)?;
            Ok(())
        });

        methods.add_meta_method(
            rlua::MetaMethod::Index,
            |lua_ctx, this, key: String| match key.as_ref() {
                "title" => Ok(this.get_title()?.to_lua(lua_ctx)?),
                "style" => Ok(WindowStyle::new(this.hwnd)?.to_lua(lua_ctx)?),
                _ => Err(Error::KeyDoesNotExist(key).into()),
            },
        );
    }
}

pub struct WindowStyle {
    hwnd: windows::HWND,
    style: u32,
    ex_style: u32,
}

unsafe impl Send for WindowStyle {}

impl WindowStyle {
    fn new(hwnd: windows::HWND) -> Result<Self> {
        Ok(WindowStyle {
            hwnd,
            style: unsafe { windows::GetWindowLong(hwnd, windows::GWL_STYLE) }? as u32,
            ex_style: unsafe { windows::GetWindowLong(hwnd, windows::GWL_EXSTYLE) }? as u32,
        })
    }

    fn get_style_flag(&self, key: String) -> Result<bool> {
        match WindowStyle::str_to_style_flag(key.as_str()) {
            Some(flag) => Ok(self.style & flag != 0),
            None => match WindowStyle::str_to_ex_style_flag(key.as_str()) {
                Some(flag) => Ok(self.ex_style & flag != 0),
                None => Err(Error::KeyDoesNotExist(key)),
            },
        }
    }

    fn set_style_flag(&mut self, key: String, val: bool) -> Result<bool> {
        match WindowStyle::str_to_style_flag(key.as_str()) {
            Some(flag) => {
                if val {
                    self.style |= flag;
                } else {
                    self.style &= !flag;
                }
                unsafe {
                    windows::SetWindowLong(
                        self.hwnd,
                        windows::GWL_STYLE,
                        self.style as windows::LONG,
                    )
                }?;
                Ok(val)
            }
            None => match WindowStyle::str_to_ex_style_flag(key.as_str()) {
                Some(flag) => {
                    if val {
                        self.ex_style |= flag;
                    } else {
                        self.ex_style &= !flag;
                    }
                    unsafe {
                        windows::SetWindowLong(
                            self.hwnd,
                            windows::GWL_STYLE,
                            self.ex_style as windows::LONG,
                        )
                    }?;
                    Ok(val)
                }
                None => Err(Error::KeyDoesNotExist(key)),
            },
        }
    }

    fn str_to_style_flag(key: &str) -> Option<u32> {
        match key {
            "border" => Some(windows::WS_BORDER),
            "caption" => Some(windows::WS_CAPTION),
            "child" => Some(windows::WS_CHILD),
            "clipchildren" => Some(windows::WS_CLIPCHILDREN),
            "clipsiblings" => Some(windows::WS_CLIPSIBLINGS),
            "disabled" => Some(windows::WS_DISABLED),
            "dlgframe" => Some(windows::WS_DLGFRAME),
            "group" => Some(windows::WS_GROUP),
            "hscroll" => Some(windows::WS_HSCROLL),
            "iconic" => Some(windows::WS_ICONIC),
            "maximize" => Some(windows::WS_MAXIMIZE),
            "maximizebox" => Some(windows::WS_MAXIMIZEBOX),
            "minimize" => Some(windows::WS_MINIMIZE),
            "minimizebox" => Some(windows::WS_MINIMIZEBOX),
            "popup" => Some(windows::WS_POPUP),
            "sysmenu" => Some(windows::WS_SYSMENU),
            "tabstop" => Some(windows::WS_TABSTOP),
            "thickframe" => Some(windows::WS_THICKFRAME),
            "visible" => Some(windows::WS_VISIBLE),
            "vscroll" => Some(windows::WS_VSCROLL),
            _ => None,
        }
    }

    fn str_to_ex_style_flag(key: &str) -> Option<u32> {
        match key {
            "acceptfiles" => Some(windows::WS_EX_ACCEPTFILES),
            "appwindow" => Some(windows::WS_EX_APPWINDOW),
            "clientedge" => Some(windows::WS_EX_CLIENTEDGE),
            "composited" => Some(windows::WS_EX_COMPOSITED),
            "contexthelp" => Some(windows::WS_EX_CONTEXTHELP),
            "controlparent" => Some(windows::WS_EX_CONTROLPARENT),
            "dlgmodalframe" => Some(windows::WS_EX_DLGMODALFRAME),
            "layered" => Some(windows::WS_EX_LAYERED),
            "layoutrtl" => Some(windows::WS_EX_LAYOUTRTL),
            "leftscrollbar" => Some(windows::WS_EX_LEFTSCROLLBAR),
            "mdichild" => Some(windows::WS_EX_MDICHILD),
            "noactivate" => Some(windows::WS_EX_NOACTIVATE),
            "noinheritlayout" => Some(windows::WS_EX_NOINHERITLAYOUT),
            "noparentnotify" => Some(windows::WS_EX_NOPARENTNOTIFY),
            "noredirectionbitmap" => Some(windows::WS_EX_NOREDIRECTIONBITMAP),
            "right" => Some(windows::WS_EX_RIGHT),
            "rtlreading" => Some(windows::WS_EX_RTLREADING),
            "staticedge" => Some(windows::WS_EX_STATICEDGE),
            "toolwindow" => Some(windows::WS_EX_TOOLWINDOW),
            "topmost" => Some(windows::WS_EX_TOPMOST),
            "transparent" => Some(windows::WS_EX_TRANSPARENT),
            "windowedge" => Some(windows::WS_EX_WINDOWEDGE),
            _ => None,
        }
    }
}

impl rlua::UserData for WindowStyle {
    fn add_methods<'lua, M: rlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(rlua::MetaMethod::Index, |_, this, key: String| {
            Ok(this.get_style_flag(key)?)
        });

        methods.add_meta_method_mut(
            rlua::MetaMethod::NewIndex,
            |_, this, args: (String, bool)| {
                let key = args.0;
                let val = args.1;
                Ok(this.set_style_flag(key, val)?)
            },
        );
    }
}

#[derive(Copy, Clone)]
pub struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl From<windows::RECT> for Rect {
    fn from(rect: windows::RECT) -> Self {
        Rect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        }
    }
}

impl From<Rect> for windows::RECT {
    fn from(rect: Rect) -> Self {
        windows::RECT {
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
                _ => Err(Error::KeyDoesNotExist(key).into()),
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
                    _ => Err(Error::KeyDoesNotExist(key).into()),
                }
            },
        );
    }
}

pub fn show_command_to_str(cmd: i32) -> Option<&'static str> {
    match cmd {
        windows::SW_FORCEMINIMIZE => Some("forceminimize"),
        windows::SW_HIDE => Some("hide"),
        windows::SW_MAXIMIZE => Some("maximize"),
        windows::SW_MINIMIZE => Some("minimize"),
        windows::SW_RESTORE => Some("restore"),
        windows::SW_SHOW => Some("show"),
        windows::SW_SHOWDEFAULT => Some("showdefault"),
        windows::SW_SHOWMINIMIZED => Some("showminimized"),
        windows::SW_SHOWMINNOACTIVE => Some("showminnoactive"),
        windows::SW_SHOWNA => Some("showna"),
        windows::SW_SHOWNOACTIVATE => Some("shownoactivate"),
        windows::SW_SHOWNORMAL => Some("shownormal"),
        _ => None,
    }
}
