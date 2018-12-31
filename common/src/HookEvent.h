#pragma once

#include <windows.h>


#pragma pack(push, 1)
struct HookEvent
{
    enum Type {
        CbtActivate,
        CbtCreateWindow,
        CbtDestroyWindow,
        CbtMinMax,
        CbtMoveSize,
    };

    Type type;
    union {
        struct {
            HWND hwnd;
            BOOL fMouse;
            HWND hWndActive;
        } cbtActivateData;
        struct {
            HWND hwnd;
            HINSTANCE hInstance;
            HMENU hMenu;
            HWND hwndParent;
            int cy;
            int cx;
            int y;
            int x;
            LONG style;
            DWORD dwExStyle;
        } cbtCreateWindowData;
        struct {
            HWND hwnd;
        } cbtDestroyWindowData;
        struct {
            HWND hwnd;
            int nCmdShow;
        } cbtMinMaxData;
        struct {
            HWND hwnd;
            RECT rect;
        } cbtMoveSizeData;
    };
};
#pragma pack(pop)
