extern crate winapi;
use winapi::ctypes;
use winapi::shared::windef;
use winapi::um::winuser;

struct HInstance {
    instance: windef::HINSTANCE,
}

enum HookId {
    CallWndProc,
    Cbt,
}

type HookProc = unsafe extern "system" fn(code: ctypes::c_int,
                                          wParam: windef::WPARAM,
                                          lParam: windef::LPARAM) -> windef::LRESULT;

struct WindowsHook {
    hook: windef::HHOOK,
}

impl WindowsHook {
    fn new(hook_id: HookId, hook_proc: HookProc, instance: &HInstance) ->

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
