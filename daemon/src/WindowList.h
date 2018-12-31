#pragma once

#include "Window.h"
#include <map>
#include <windows.h>
#include <optional>
// #include "HookListener.h"

class HookManager;

class WindowList // : public HookListener
{
    friend BOOL CALLBACK window_list_enum_proc(HWND, LPARAM);

public:
    WindowList(HookManager &hook_manager);

private:
    void add_window(HWND hwnd);

    std::map<HWND, Window> hwnd_map_;
    std::optional<std::string> enum_error_;
};
