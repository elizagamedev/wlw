#include <windows.h>
#include "HookEvent.h"

#pragma data_seg(push, "shared")
__declspec(dllexport) uint32_t server_pid = 0;
#pragma data_seg(pop)
#pragma comment(linker, "/section:shared,RWS")

BOOL WINAPI DllMain(
    HINSTANCE hinstDLL,
    DWORD fdwReason,
    LPVOID lpvReserved)
{
    switch (fdwReason) {
    case DLL_PROCESS_ATTACH:
        DisableThreadLibraryCalls(hinstDLL);
        return TRUE;
    case DLL_PROCESS_DETACH:
        return TRUE;
    }
    return FALSE;
}

__declspec(dllexport) LRESULT CALLBACK callwndproc_proc(int nCode, WPARAM wParam, LPARAM lParam)
{
    const CWPSTRUCT *cwp = (const CWPSTRUCT *)lParam;
    HookEvent event;
    switch (cwp->message) {
    case WM_SIZE:
        event.type = TYPE_CWP_SIZE;
        event.cwpSizeData.hwnd = (PortableHWND)(intptr_t)cwp->hwnd;
        event.cwpSizeData.size = (PortableDWORD)cwp->wParam;
        break;
    default:
        return CallNextHookEx(NULL, nCode, wParam, lParam);
    }
    /* COPYDATASTRUCT cds; */
    /* cds.dwData = 0xDEADBEEF; */
    /* cds.cbData = sizeof(event); */
    /* cds.lpData = &event; */
    /* SendMessageW(daemon_window, WM_COPYDATA, */
    /*              (WPARAM)daemon_window, */
    /*              (LPARAM)&cds); */
    return CallNextHookEx(NULL, nCode, wParam, lParam);
}

__declspec(dllexport) LRESULT CALLBACK cbt_proc(int nCode, WPARAM wParam, LPARAM lParam)
{
    HookEvent event;
    switch (nCode) {
    case HCBT_ACTIVATE: {
        const CBTACTIVATESTRUCT *cbtas
            = (const CBTACTIVATESTRUCT *)lParam;
        event.type = TYPE_CBT_ACTIVATE;
        event.cbtActivateData.hwnd = (PortableHWND)(intptr_t)wParam;
        event.cbtActivateData.fMouse = (PortableBOOL)cbtas->fMouse;
        event.cbtActivateData.hWndActive = (PortableHWND)(intptr_t)cbtas->hWndActive;
    } break;
    case HCBT_CREATEWND: {
        const CREATESTRUCTW *lpcs
            = ((const CBT_CREATEWNDW *)lParam)->lpcs;
        event.type = TYPE_CBT_CREATE_WINDOW;
        event.cbtCreateWindowData.hwnd = (PortableHWND)(intptr_t)wParam;
        event.cbtCreateWindowData.hInstance = (PortableHINSTANCE)(intptr_t)lpcs->hInstance;
        event.cbtCreateWindowData.hMenu = (PortableHMENU)(intptr_t)lpcs->hMenu;
        event.cbtCreateWindowData.hwndParent = (PortableHWND)(intptr_t)lpcs->hwndParent;
        event.cbtCreateWindowData.cy = (PortableInt)lpcs->cy;
        event.cbtCreateWindowData.cx = (PortableInt)lpcs->cx;
        event.cbtCreateWindowData.y = (PortableInt)lpcs->y;
        event.cbtCreateWindowData.x = (PortableInt)lpcs->x;
        event.cbtCreateWindowData.style = (PortableLONG)lpcs->style;
        event.cbtCreateWindowData.dwExStyle = (PortableDWORD)lpcs->dwExStyle;
    } break;
    case HCBT_DESTROYWND: {
        event.type = TYPE_CBT_DESTROY_WINDOW;
        event.cbtDestroyWindowData.hwnd = (PortableHWND)(intptr_t)wParam;
    } break;
    case HCBT_MINMAX: {
        event.type = TYPE_CBT_MIN_MAX;
        event.cbtMinMaxData.hwnd = (PortableHWND)(intptr_t)wParam;
        event.cbtMinMaxData.nCmdShow = (PortableInt)LOWORD((DWORD)lParam);
    } break;
    case HCBT_MOVESIZE: {
        event.type = TYPE_CBT_MOVE_SIZE;
        event.cbtMoveSizeData.hwnd = (PortableHWND)(intptr_t)wParam;
        const RECT *rect = (const RECT *)lParam;
        event.cbtMoveSizeData.rect.left = (PortableInt)rect->left;
        event.cbtMoveSizeData.rect.top = (PortableInt)rect->top;
        event.cbtMoveSizeData.rect.right = (PortableInt)rect->right;
        event.cbtMoveSizeData.rect.bottom = (PortableInt)rect->bottom;
    } break;
    default:
        return CallNextHookEx(NULL, nCode, wParam, lParam);
    }
    /* COPYDATASTRUCT cds; */
    /* cds.dwData = 0xDEADBEEF; */
    /* cds.cbData = sizeof(event); */
    /* cds.lpData = &event; */
    /* SendMessageW(daemon_window, WM_COPYDATA, */
    /*              (WPARAM)daemon_window, */
    /*              (LPARAM)&cds); */
    return CallNextHookEx(NULL, nCode, wParam, lParam);
}
