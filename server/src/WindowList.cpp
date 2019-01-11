#include "WindowList.h"
#include "HookManager.h"
#include <exception>

BOOL CALLBACK WindowList::window_enum_proc(HWND hwnd, LPARAM lParam)
{
    WindowList *wl = reinterpret_cast<WindowList *>(lParam);
    try {
        wl->add_window(hwnd);
    } catch (...) {
        wl->enum_outcome_ = std::current_exception();
        return FALSE;
    }
    return TRUE;
}

outcome::checked<std::shared_ptr<WindowList>, WindowsError> WindowList::create(HookManager &hook_manager)
{
    std::shared_ptr<WindowList> wl(new WindowList());
    if (!EnumWindows(window_enum_proc, reinterpret_cast<LPARAM>(wl.get()))) {
        // GetLastError() has a chance of being 0 here if an exception was thrown
        DWORD last_error = GetLastError();
        if (last_error != ERROR_SUCCESS) {
            return WindowsError(last_error);
        }
    }
    // Enum outcome potentially has an error
    if (wl->enum_outcome_.has_value()) {
        auto &outcome = wl->enum_outcome_.value();
        if (outcome.has_error()) {
            return outcome.error();
        } else if (outcome.has_exception()) {
            std::rethrow_exception(outcome.exception());
        }
    }
    // Register with the hook manager
    hook_manager.register_listener(wl);
    return wl;
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
