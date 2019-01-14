#pragma once

#include <windows.h>
#include <stdint.h>

enum Type {
    TYPE_CWP_SIZE,
    TYPE_CBT_ACTIVATE,
    TYPE_CBT_CREATE_WINDOW,
    TYPE_CBT_DESTROY_WINDOW,
    TYPE_CBT_MIN_MAX,
    TYPE_CBT_MOVE_SIZE,
};

typedef uint8_t PortableBOOL;
typedef uint32_t PortableDWORD;
typedef uint32_t PortableHWND;
typedef uint32_t PortableHINSTANCE;
typedef uint32_t PortableHMENU;
typedef uint32_t PortableLONG;
typedef int32_t PortableInt;

#pragma pack(push, 1)
struct _PortableRECT {
    PortableLONG left;
    PortableLONG top;
    PortableLONG right;
    PortableLONG bottom;
};
typedef struct _PortableRECT PortableRECT;

struct _HookEvent {
    uint8_t type;
    union {
        struct {
            PortableHWND hwnd;
            PortableDWORD size;
        } cwpSizeData;
        struct {
            PortableHWND hwnd;
            PortableBOOL fMouse;
            PortableHWND hWndActive;
        } cbtActivateData;
        struct {
            PortableHWND hwnd;
            PortableHINSTANCE hInstance;
            PortableHMENU hMenu;
            PortableHWND hwndParent;
            PortableInt cy;
            PortableInt cx;
            PortableInt y;
            PortableInt x;
            PortableLONG style;
            PortableDWORD dwExStyle;
        } cbtCreateWindowData;
        struct {
            PortableHWND hwnd;
        } cbtDestroyWindowData;
        struct {
            PortableHWND hwnd;
            PortableInt nCmdShow;
        } cbtMinMaxData;
        struct {
            PortableHWND hwnd;
            PortableRECT rect;
        } cbtMoveSizeData;
    };
};
typedef struct _HookEvent HookEvent;
#pragma pack(pop)
