#pragma once

#include "HookListener.h"
#include "Window.h"
#include <map>
#include <optional>
#include <windows.h>

class HookManager;

class WindowList : public HookListener
{
    friend BOOL CALLBACK window_list_enum_proc(HWND, LPARAM);

public:
    WindowList(HookManager &hook_manager);

    void handle_events();

private:
    void add_window(HWND hwnd);
    void remove_window(HWND hwnd);

    std::map<HWND, Window> hwnd_map_;
    std::optional<std::string> enum_error_;
};
