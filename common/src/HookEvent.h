#pragma once

#include <windows.h>


#pragma pack(push, 1)
struct HookEvent
{
    int idHook;
    int nCode;
    WPARAM wParam;
    LPARAM lParam;
};
#pragma pack(pop)
