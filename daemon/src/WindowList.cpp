#include "WindowList.h"
#include "win32.h"
#include "HookManager.h"


BOOL CALLBACK window_list_enum_proc(HWND hwnd, LPARAM lParam)
{
    WindowList *wl = reinterpret_cast<WindowList*>(lParam);
    try {
        wl->add_window(hwnd);
    } catch (const std::exception &e) {
        wl->enum_error_ = e.what();
    }
    return TRUE;
}

WindowList::WindowList(HookManager &hook_manager)
    : enum_error_(std::nullopt)
{
    if (!EnumWindows(window_list_enum_proc, reinterpret_cast<LPARAM>(this))) {
        throw win32::get_last_error_exception();
    }
    if (enum_error_.has_value()) {
        throw std::runtime_error(enum_error_.value());
    }
    // Register with the hook manager
    hook_manager.register_listener(this);
}

void WindowList::add_window(HWND hwnd)
{
    hwnd_map_.insert(std::pair<HWND, Window>(hwnd, Window(hwnd)));
}
