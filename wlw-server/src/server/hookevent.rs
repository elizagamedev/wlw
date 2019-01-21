#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(non_snake_case)]
use winapi::ctypes::*;
use winapi::shared::minwindef::DWORD;
use winapi::shared::ntdef::LONG;
use winapi::shared::windef::{HWND, RECT};

pub type PortableBOOL = u8;
pub type PortableDWORD = u32;
pub type PortableHWND = u32;
pub type PortableLONG = i32;
pub type PortableInt = i32;

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct PortableRECT {
    left: PortableLONG,
    top: PortableLONG,
    right: PortableLONG,
    bottom: PortableLONG,
}

impl From<RECT> for PortableRECT {
    fn from(rect: RECT) -> Self {
        PortableRECT {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        }
    }
}

impl From<PortableRECT> for RECT {
    fn from(rect: PortableRECT) -> Self {
        RECT {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        }
    }
}

pub enum HookEvent {
    CwpShowWindow { hwnd: HWND, shown: bool },
    CbtActivate { hwnd: HWND, caused_by_mouse: bool },
    CbtCreateWindow { hwnd: HWND, rect: RECT },
    CbtDestroyWindow { hwnd: HWND },
    CbtMinMax { hwnd: HWND, show_command: c_int },
    CbtMoveSize { hwnd: HWND, rect: RECT },
}

impl From<HookEventC> for HookEvent {
    fn from(hec: HookEventC) -> Self {
        unsafe {
            match hec.kind {
                TYPE_CWP_SHOW_WINDOW => HookEvent::CwpShowWindow {
                    hwnd: hec.u.cwp_show_window_data.hwnd as HWND,
                    shown: hec.u.cwp_show_window_data.shown != 0,
                },
                TYPE_CBT_ACTIVATE => HookEvent::CbtActivate {
                    hwnd: hec.u.cbt_activate_data.hwnd as HWND,
                    caused_by_mouse: hec.u.cbt_activate_data.caused_by_mouse != 0,
                },
                TYPE_CBT_CREATE_WINDOW => HookEvent::CbtCreateWindow {
                    hwnd: hec.u.cbt_create_window_data.hwnd as HWND,
                    rect: RECT::from(hec.u.cbt_create_window_data.rect),
                },
                TYPE_CBT_DESTROY_WINDOW => HookEvent::CbtDestroyWindow {
                    hwnd: hec.u.cbt_destroy_window_data.hwnd as HWND,
                },
                TYPE_CBT_MIN_MAX => HookEvent::CbtMinMax {
                    hwnd: hec.u.cbt_min_max_data.hwnd as HWND,
                    show_command: hec.u.cbt_min_max_data.show_command,
                },
                TYPE_CBT_MOVE_SIZE => HookEvent::CbtMoveSize {
                    hwnd: hec.u.cbt_move_size_data.hwnd as HWND,
                    rect: RECT::from(hec.u.cbt_move_size_data.rect),
                },
                _ => unreachable!(),
            }
        }
    }
}

const TYPE_CWP_SHOW_WINDOW: u8 = 0;
const TYPE_CBT_ACTIVATE: u8 = 1;
const TYPE_CBT_CREATE_WINDOW: u8 = 2;
const TYPE_CBT_DESTROY_WINDOW: u8 = 3;
const TYPE_CBT_MIN_MAX: u8 = 4;
const TYPE_CBT_MOVE_SIZE: u8 = 5;

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct HookEventC {
    kind: u8,
    u: HookEventUnion,
}

#[repr(packed)]
#[derive(Copy, Clone)]
union HookEventUnion {
    cwp_show_window_data: CwpShowWindowData,
    cbt_activate_data: CbtActivateData,
    cbt_create_window_data: CbtCreateWindowData,
    cbt_destroy_window_data: CbtDestroyWindowData,
    cbt_min_max_data: CbtMinMaxData,
    cbt_move_size_data: CbtMoveSizeData,
}

#[repr(packed)]
#[derive(Copy, Clone)]
struct CwpShowWindowData {
    hwnd: PortableHWND,
    shown: PortableBOOL,
}

#[repr(packed)]
#[derive(Copy, Clone)]
struct CbtActivateData {
    hwnd: PortableHWND,
    caused_by_mouse: PortableBOOL,
}

#[repr(packed)]
#[derive(Copy, Clone)]
struct CbtCreateWindowData {
    hwnd: PortableHWND,
    rect: PortableRECT,
}

#[repr(packed)]
#[derive(Copy, Clone)]
struct CbtDestroyWindowData {
    hwnd: PortableHWND,
}

#[repr(packed)]
#[derive(Copy, Clone)]
struct CbtMinMaxData {
    hwnd: PortableHWND,
    show_command: PortableInt,
}

#[repr(packed)]
#[derive(Copy, Clone)]
struct CbtMoveSizeData {
    hwnd: PortableHWND,
    rect: PortableRECT,
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub union HookResponse {
    pub pos_and_size_data: PosAndSizeData,
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct PosAndSizeData {
    pub rect: PortableRECT,
}
