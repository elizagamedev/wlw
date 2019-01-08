#include "hooks.h"
#include "win32.h"

namespace hooks
{
#pragma data_seg(push, "shared")
    DWORD daemon_process_id = 0;
#pragma data_seg(pop)
#pragma comment(linker, "/section:shared,RWS")

    HANDLE daemon_process_handle = NULL;
    HWND daemon_window = NULL;

    static BOOL CALLBACK enum_windows_proc(HWND hwnd, LPARAM lParam)
    {
        DWORD process_id;
        GetWindowThreadProcessId(hwnd, &process_id);
        if (process_id == hooks::daemon_process_id) {
            hooks::daemon_window = hwnd;
            return FALSE;
        }
        return TRUE;
    }

    void connect()
    {
        if (daemon_process_id == 0) {
            throw std::runtime_error("Process ID is 0");
        }
        // Find the daemon's window and open its process handle.
        EnumWindows(enum_windows_proc, 0);
        daemon_process_handle
            = OpenProcess(SYNCHRONIZE, FALSE, daemon_process_id);
        if (daemon_process_handle == NULL) {
            throw win32::get_last_error_exception();
        }
        // Sanity check to make sure we're connected
        if (!is_connected(0)) {
            throw std::runtime_error("Could not connect to daemon");
        }
    }

    void disconnect()
    {
        CloseHandle(daemon_process_handle);
        daemon_process_handle = NULL;
        daemon_window = NULL;
    }

    bool is_connected(int ms)
    {
        if (daemon_process_id == 0) {
            return false;
        }
        if (daemon_process_handle == NULL) {
            return false;
        }
        if (daemon_window == NULL) {
            return false;
        }
        return WaitForSingleObject(daemon_process_handle, ms) == WAIT_TIMEOUT;
    }

    LRESULT CALLBACK callwndproc_proc(int nCode, WPARAM wParam, LPARAM lParam)
    {
        const CWPSTRUCT *cwp = reinterpret_cast<const CWPSTRUCT *>(lParam);
        HookEvent event;
        switch (cwp->message) {
        case WM_SIZE:
            event.type = HookEvent::CwpSize;
            event.cwpSizeData.hwnd = cwp->hwnd;
            event.cwpSizeData.size = static_cast<DWORD>(cwp->wParam);
            break;
        default:
            return CallNextHookEx(NULL, nCode, wParam, lParam);
        }
        COPYDATASTRUCT cds;
        cds.dwData = 0xDEADBEEF;
        cds.cbData = sizeof(event);
        cds.lpData = &event;
        SendMessageW(daemon_window, WM_COPYDATA,
                     reinterpret_cast<WPARAM>(daemon_window),
                     reinterpret_cast<LPARAM>(&cds));
        return CallNextHookEx(NULL, nCode, wParam, lParam);
    }

    LRESULT CALLBACK cbt_proc(int nCode, WPARAM wParam, LPARAM lParam)
    {
        HookEvent event;
        switch (nCode) {
        case HCBT_ACTIVATE: {
            const CBTACTIVATESTRUCT *cbtas
                = reinterpret_cast<const CBTACTIVATESTRUCT *>(lParam);
            event.type = HookEvent::CbtActivate;
            event.cbtActivateData.hwnd = reinterpret_cast<HWND>(wParam);
            event.cbtActivateData.fMouse = cbtas->fMouse;
            event.cbtActivateData.hWndActive = cbtas->hWndActive;
        } break;
        case HCBT_CREATEWND: {
            const CREATESTRUCTW *lpcs
                = reinterpret_cast<const CBT_CREATEWNDW *>(lParam)->lpcs;
            event.type = HookEvent::CbtCreateWindow;
            event.cbtCreateWindowData.hwnd = reinterpret_cast<HWND>(wParam);
            event.cbtCreateWindowData.hInstance = lpcs->hInstance;
            event.cbtCreateWindowData.hMenu = lpcs->hMenu;
            event.cbtCreateWindowData.hwndParent = lpcs->hwndParent;
            event.cbtCreateWindowData.cy = lpcs->cy;
            event.cbtCreateWindowData.cx = lpcs->cx;
            event.cbtCreateWindowData.y = lpcs->y;
            event.cbtCreateWindowData.x = lpcs->x;
            event.cbtCreateWindowData.style = lpcs->style;
            event.cbtCreateWindowData.dwExStyle = lpcs->dwExStyle;
        } break;
        case HCBT_DESTROYWND: {
            event.type = HookEvent::CbtDestroyWindow;
            event.cbtDestroyWindowData.hwnd = reinterpret_cast<HWND>(wParam);
        } break;
        case HCBT_MINMAX: {
            event.type = HookEvent::CbtMinMax;
            event.cbtMinMaxData.hwnd = reinterpret_cast<HWND>(wParam);
            event.cbtMinMaxData.nCmdShow = LOWORD(static_cast<DWORD>(lParam));
        } break;
        case HCBT_MOVESIZE: {
            event.type = HookEvent::CbtMoveSize;
            event.cbtMoveSizeData.hwnd = reinterpret_cast<HWND>(wParam);
            event.cbtMoveSizeData.rect
                = *reinterpret_cast<const RECT *>(lParam);
        } break;
        default:
            return CallNextHookEx(NULL, nCode, wParam, lParam);
        }
        COPYDATASTRUCT cds;
        cds.dwData = 0xDEADBEEF;
        cds.cbData = sizeof(event);
        cds.lpData = &event;
        SendMessageW(daemon_window, WM_COPYDATA,
                     reinterpret_cast<WPARAM>(daemon_window),
                     reinterpret_cast<LPARAM>(&cds));
        return CallNextHookEx(NULL, nCode, wParam, lParam);
    }
}
