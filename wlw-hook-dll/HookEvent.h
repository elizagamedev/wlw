#pragma once

#include <stdint.h>
#include <windows.h>

enum Kind {
    KIND_CWP_SHOW_WINDOW,
    KIND_CBT_ACTIVATE,
    KIND_CBT_CREATE_WINDOW,
    KIND_CBT_DESTROY_WINDOW,
    KIND_CBT_MIN_MAX,
    KIND_CBT_MOVE_SIZE,
};

typedef uint8_t PortableBOOL;
typedef uint32_t PortableDWORD;
typedef uint32_t PortableHWND;
typedef int32_t PortableLONG;
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
    uint8_t kind;
    union {
        struct {
            PortableHWND hwnd;
            PortableBOOL shown;
        } cwp_show_window_data;
        struct {
            PortableHWND hwnd;
            PortableBOOL caused_by_mouse;
        } cbt_activate_data;
        struct {
            PortableHWND hwnd;
            PortableRECT rect;
        } cbt_create_window_data;
        struct {
            PortableHWND hwnd;
        } cbt_destroy_window_data;
        struct {
            PortableHWND hwnd;
            PortableInt show_command;
        } cbt_min_max_data;
        struct {
            PortableHWND hwnd;
            PortableRECT rect;
        } cbt_move_size_data;
    };
};
typedef struct _HookEvent HookEvent;

struct _HookResponse {
    union {
        struct {
            PortableRECT rect;
        } pos_and_size_data;
    };
};
typedef struct _HookResponse HookResponse;

#pragma pack(pop)
