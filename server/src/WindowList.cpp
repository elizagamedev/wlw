#include "WindowList.h"
#include "HookManager.h"
#include "win32.h"

BOOL CALLBACK window_list_enum_proc(HWND hwnd, LPARAM lParam)
{
    WindowList *wl = reinterpret_cast<WindowList *>(lParam);
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

void WindowList::handle_events()
{
    consume_all_hook_events([this](HookEvent &e) {
        switch (e.type) {
        case HookEvent::CbtCreateWindow:
            add_window(e.cbtCreateWindowData.hwnd);
            break;
        case HookEvent::CbtDestroyWindow:
            remove_window(e.cbtDestroyWindowData.hwnd);
            break;
        }
    });
}

void WindowList::add_window(HWND hwnd)
{
    hwnd_map_.insert(std::pair<HWND, Window>(hwnd, Window(hwnd)));
}

void WindowList::remove_window(HWND hwnd)
{
    auto it = hwnd_map_.find(hwnd);
    if (it != hwnd_map_.end()) {
        hwnd_map_.erase(it);
    }
}
