#include "HookEvent.h"
#include <stdio.h>
#include <windows.h>

#pragma data_seg(push, "shared")
uint32_t server_pid = 0;
#pragma data_seg(pop)
#pragma comment(linker, "/section:shared,RWS")

volatile LONG ready = FALSE;
HANDLE pipe = INVALID_HANDLE_VALUE;
CRITICAL_SECTION mutex;

#define PIPE_NAME_BUF_LEN 256

BOOL WINAPI DllMain(HINSTANCE hinstDLL, DWORD fdwReason, LPVOID lpvReserved) {
    switch (fdwReason) {
    case DLL_PROCESS_ATTACH: {
        DisableThreadLibraryCalls(hinstDLL);
        if (server_pid == 0) {
            // PID has not been initialized; this is the hook process
            return TRUE;
        }
        WCHAR pipe_name_buf[PIPE_NAME_BUF_LEN];
        swprintf_s(pipe_name_buf, PIPE_NAME_BUF_LEN,
                   L"\\\\.\\pipe\\wlw_server_%d", server_pid);
        pipe = CreateFileW(pipe_name_buf, GENERIC_READ | GENERIC_WRITE, 0, NULL,
                           OPEN_EXISTING, 0, NULL);
        if (pipe == INVALID_HANDLE_VALUE) {
            return FALSE;
        }
        DWORD mode = PIPE_READMODE_MESSAGE;
        if (!SetNamedPipeHandleState(pipe, &mode, NULL, NULL)) {
            CloseHandle(pipe);
            return FALSE;
        }
        InitializeCriticalSection(&mutex);
        InterlockedExchange(&ready, TRUE);
        return TRUE;
    }
    case DLL_PROCESS_DETACH: {
        if (ready) {
            EnterCriticalSection(&mutex);
            InterlockedExchange(&ready, FALSE);
            LeaveCriticalSection(&mutex);
            DeleteCriticalSection(&mutex);
            if (pipe != INVALID_HANDLE_VALUE) {
                CloseHandle(pipe);
            }
        }
        return TRUE;
    }
    }
    return FALSE;
}

inline BOOL transact(HookEvent *send, HookResponse *recv) {
    EnterCriticalSection(&mutex);

    DWORD num_read;
    BOOL ret
        = TransactNamedPipe(pipe, (LPVOID)send, sizeof(HookEvent), (LPVOID)recv,
                            sizeof(HookResponse), &num_read, NULL);

    if (ret) {
        LeaveCriticalSection(&mutex);
        return TRUE;
    } else {
        InterlockedExchange(&ready, FALSE);
        CloseHandle(pipe);
        pipe = INVALID_HANDLE_VALUE;
        LeaveCriticalSection(&mutex);
        return FALSE;
    }
}

inline void write(HookEvent *send) {
    EnterCriticalSection(&mutex);

    DWORD num_sent;
    BOOL ret
        = WriteFile(pipe, (LPCVOID)send, sizeof(HookEvent), &num_sent, NULL);

    if (ret) {
        LeaveCriticalSection(&mutex);
    } else {
        InterlockedExchange(&ready, FALSE);
        CloseHandle(pipe);
        pipe = INVALID_HANDLE_VALUE;
        LeaveCriticalSection(&mutex);
    }
}

inline BOOL is_worthy_window(HWND hwnd,
                             BOOL exclude_top_level,
                             BOOL exclude_hidden,
                             BOOL exclude_ws_caption) {
    if (exclude_top_level && hwnd != GetAncestor(hwnd, GA_ROOT)) {
        return FALSE;
    }
    if (exclude_hidden && !IsWindowVisible(hwnd)) {
        return FALSE;
    }
    LONG style = GetWindowLongW(hwnd, GWL_STYLE);
    LONG ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
    if (!(style & WS_CAPTION) || (ex_style & WS_EX_TOOLWINDOW)) {
        return FALSE;
    }
    return TRUE;
}

LRESULT CALLBACK callwndproc_proc(int nCode, WPARAM wParam, LPARAM lParam) {
    if (ready) {
        const CWPSTRUCT *cwp = (const CWPSTRUCT *)lParam;
        switch (cwp->message) {
        case WM_SHOWWINDOW: {
            if (cwp->lParam == 0
                && is_worthy_window(cwp->hwnd, TRUE, FALSE, TRUE)) {
                // lParam == 0 indicates that the window was hidden/shown via
                // ShowWindow
                HookEvent event;
                event.kind = KIND_CWP_SHOW_WINDOW;
                event.cwp_show_window_data.hwnd
                    = (PortableHWND)(intptr_t)cwp->hwnd;
                event.cwp_show_window_data.shown
                    = (PortableHWND)(intptr_t)cwp->wParam;
                write(&event);
            }
        } break;
        }
    }

    return CallNextHookEx(NULL, nCode, wParam, lParam);
}

LRESULT CALLBACK cbt_proc(int nCode, WPARAM wParam, LPARAM lParam) {
    if (ready) {
        switch (nCode) {
        case HCBT_ACTIVATE: {
            HookEvent event;
            if (is_worthy_window((HWND)wParam, TRUE, TRUE, TRUE)) {
                const CBTACTIVATESTRUCT *cbtas
                    = (const CBTACTIVATESTRUCT *)lParam;
                event.kind = KIND_CBT_ACTIVATE;
                event.cbt_activate_data.hwnd = (PortableHWND)(intptr_t)wParam;
                event.cbt_activate_data.caused_by_mouse
                    = (PortableBOOL)cbtas->fMouse;
                write(&event);
            }
        } break;
        case HCBT_CREATEWND: {
            HookEvent event;
            CREATESTRUCTW *lpcs = ((const CBT_CREATEWNDW *)lParam)->lpcs;
            if ((lpcs->style & WS_CAPTION) && !(lpcs->style & WS_CHILD)
                && !(lpcs->dwExStyle & WS_EX_TOOLWINDOW)) {
                event.kind = KIND_CBT_CREATE_WINDOW;
                event.cbt_create_window_data.hwnd
                    = (PortableHWND)(intptr_t)wParam;
                event.cbt_create_window_data.rect.bottom
                    = (PortableLONG)(lpcs->cy + lpcs->y);
                event.cbt_create_window_data.rect.right
                    = (PortableLONG)(lpcs->cx + lpcs->x);
                event.cbt_create_window_data.rect.top = (PortableLONG)lpcs->y;
                event.cbt_create_window_data.rect.left = (PortableLONG)lpcs->x;
                HookResponse response;
                if (transact(&event, &response)) {
                    lpcs->cy = (int)(response.pos_and_size_data.rect.bottom
                                     - response.pos_and_size_data.rect.top);
                    lpcs->cx = (int)(response.pos_and_size_data.rect.right
                                     - response.pos_and_size_data.rect.left);
                    lpcs->y = (int)response.pos_and_size_data.rect.top;
                    lpcs->x = (int)response.pos_and_size_data.rect.left;
                }
            }
        } break;
        case HCBT_DESTROYWND: {
            // There is no satisfying way to select for messages that we care
            // about here. Messages will be filtered out server-side.
            HookEvent event;
            event.kind = KIND_CBT_DESTROY_WINDOW;
            event.cbt_destroy_window_data.hwnd = (PortableHWND)(intptr_t)wParam;
            write(&event);
        } break;
        case HCBT_MINMAX: {
            if (is_worthy_window((HWND)wParam, TRUE, TRUE, TRUE)) {
                HookEvent event;
                event.kind = KIND_CBT_MIN_MAX;
                event.cbt_min_max_data.hwnd = (PortableHWND)(intptr_t)wParam;
                event.cbt_min_max_data.show_command
                    = (PortableInt)LOWORD((DWORD)lParam);
                write(&event);
            }
        } break;
        case HCBT_MOVESIZE: {
            if (is_worthy_window((HWND)wParam, TRUE, TRUE, TRUE)) {
                HookEvent event;
                event.kind = KIND_CBT_MOVE_SIZE;
                event.cbt_move_size_data.hwnd = (PortableHWND)(intptr_t)wParam;
                RECT *rect = (RECT *)lParam;
                event.cbt_move_size_data.rect.left = (PortableLONG)rect->left;
                event.cbt_move_size_data.rect.top = (PortableLONG)rect->top;
                event.cbt_move_size_data.rect.right = (PortableLONG)rect->right;
                event.cbt_move_size_data.rect.bottom
                    = (PortableLONG)rect->bottom;
                HookResponse response;
                if (transact(&event, &response)) {
                    rect->left = (LONG)response.pos_and_size_data.rect.left;
                    rect->top = (LONG)response.pos_and_size_data.rect.top;
                    rect->right = (LONG)response.pos_and_size_data.rect.right;
                    rect->bottom = (LONG)response.pos_and_size_data.rect.bottom;
                }
            }
        } break;
        }
    }
    return CallNextHookEx(NULL, nCode, wParam, lParam);
}
