#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(non_snake_case)]
use winapi::ctypes::*;

pub type PortableBOOL = u8;
pub type PortableDWORD = u32;
pub type PortableHWND = u32;
pub type PortableHINSTANCE = u32;
pub type PortableHMENU = u32;
pub type PortableLONG = u32;
pub type PortableInt = i32;

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct PortableRECT {
    left: PortableLONG,
    top: PortableLONG,
    right: PortableLONG,
    bottom: PortableLONG,
}

const TYPE_CWP_SIZE: u8 = 0;
const TYPE_CBT_ACTIVATE: u8 = 1;
const TYPE_CBT_CREATE_WINDOW: u8 = 2;
const TYPE_CBT_DESTROY_WINDOW: u8 = 3;
const TYPE_CBT_MIN_MAX: u8 = 4;
const TYPE_CBT_MOVE_SIZE: u8 = 5;

pub enum HookEvent {
    CwpSize {
        hwnd: PortableHWND,
        size: PortableDWORD,
    },
    CbtActivate {
        hwnd: PortableHWND,
        fMouse: PortableBOOL,
        hWndActive: PortableHWND,
    },
    CbtCreateWindow {
        hwnd: PortableHWND,
        hInstance: PortableHINSTANCE,
        hMenu: PortableHMENU,
        hwndParent: PortableHWND,
        cy: PortableInt,
        cx: PortableInt,
        y: PortableInt,
        x: PortableInt,
        style: PortableLONG,
        dwExStyle: PortableDWORD,
    },
    CbtDestroyWindow {
        hwnd: PortableHWND,
    },
    CbtMinMax {
        hwnd: PortableHWND,
        nCmdShow: PortableInt,
    },
    CbtMoveSize {
        hwnd: PortableHWND,
        rect: PortableRECT,
    },
}

impl From<HookEventC> for HookEvent {
    fn from(hec: HookEventC) -> Self {
        unsafe {
            match hec.kind {
                TYPE_CWP_SIZE => HookEvent::CwpSize {
                    hwnd: hec.u.cwpSizeData.hwnd,
                    size: hec.u.cwpSizeData.size,
                },
                TYPE_CBT_ACTIVATE => HookEvent::CbtActivate {
                    hwnd: hec.u.cbtActivateData.hwnd,
                    fMouse: hec.u.cbtActivateData.fMouse,
                    hWndActive: hec.u.cbtActivateData.hWndActive,
                },
                TYPE_CBT_CREATE_WINDOW => HookEvent::CbtCreateWindow {
                    hwnd: hec.u.cbtCreateWindowData.hwnd,
                    hInstance: hec.u.cbtCreateWindowData.hInstance,
                    hMenu: hec.u.cbtCreateWindowData.hMenu,
                    hwndParent: hec.u.cbtCreateWindowData.hwndParent,
                    cy: hec.u.cbtCreateWindowData.cy,
                    cx: hec.u.cbtCreateWindowData.cx,
                    y: hec.u.cbtCreateWindowData.y,
                    x: hec.u.cbtCreateWindowData.x,
                    style: hec.u.cbtCreateWindowData.style,
                    dwExStyle: hec.u.cbtCreateWindowData.dwExStyle,
                },
                TYPE_CBT_DESTROY_WINDOW => HookEvent::CbtDestroyWindow {
                    hwnd: hec.u.cbtDestroyWindowData.hwnd,
                },
                TYPE_CBT_MIN_MAX => HookEvent::CbtMinMax {
                    hwnd: hec.u.cbtMinMaxData.hwnd,
                    nCmdShow: hec.u.cbtMinMaxData.nCmdShow,
                },
                TYPE_CBT_MOVE_SIZE => HookEvent::CbtMoveSize {
                    hwnd: hec.u.cbtMoveSizeData.hwnd,
                    rect: hec.u.cbtMoveSizeData.rect,
                },
                _ => unreachable!(),
            }
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct HookEventC {
    kind: u8,
    u: HookEventUnion,
}

#[repr(packed)]
#[derive(Copy, Clone)]
union HookEventUnion {
    cwpSizeData: CwpSizeData,
    cbtActivateData: CbtActivateData,
    cbtCreateWindowData: CbtCreateWindowData,
    cbtDestroyWindowData: CbtDestroyWindowData,
    cbtMinMaxData: CbtMinMaxData,
    cbtMoveSizeData: CbtMoveSizeData,
}

#[repr(packed)]
#[derive(Copy, Clone)]
struct CwpSizeData {
    hwnd: PortableHWND,
    size: PortableDWORD,
}

#[repr(packed)]
#[derive(Copy, Clone)]
struct CbtActivateData {
    hwnd: PortableHWND,
    fMouse: PortableBOOL,
    hWndActive: PortableHWND,
}

#[repr(packed)]
#[derive(Copy, Clone)]
struct CbtCreateWindowData {
    hwnd: PortableHWND,
    hInstance: PortableHINSTANCE,
    hMenu: PortableHMENU,
    hwndParent: PortableHWND,
    cy: PortableInt,
    cx: PortableInt,
    y: PortableInt,
    x: PortableInt,
    style: PortableLONG,
    dwExStyle: PortableDWORD,
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
    nCmdShow: PortableInt,
}

#[repr(packed)]
#[derive(Copy, Clone)]
struct CbtMoveSizeData {
    hwnd: PortableHWND,
    rect: PortableRECT,
}
