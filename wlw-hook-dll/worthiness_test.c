#include <stdio.h>
#include <windows.h>

inline BOOL is_worthy_window(HWND hwnd) {
    if (hwnd != GetAncestor(hwnd, GA_ROOT)) {
        return FALSE;
    }
    if (!IsWindowVisible(hwnd)) {
        return FALSE;
    }
    LONG style = GetWindowLongW(hwnd, GWL_STYLE);
    if (!(style & WS_CAPTION)) {
        return FALSE;
    }
    return TRUE;
}

BOOL CALLBACK enum_windows_proc(HWND hwnd, LPARAM lParam) {
    if (!is_worthy_window(hwnd)) {
        return TRUE;
    }
    char buffer[256];
    GetWindowTextA(hwnd, buffer, 256);
    printf("%s", buffer);
    GetClassNameA(hwnd, buffer, 256);
    printf(" : %s\n", buffer);
    return TRUE;
}

int main() {
    EnumWindows(enum_windows_proc, 0);
}
