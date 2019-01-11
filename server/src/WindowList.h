#pragma once

#include "HookListener.h"
#include "Window.h"
#include <map>
#include <windows.h>
#include <outcome.hpp>
#include "WindowsError.h"
#include <memory>
#include <optional>

namespace outcome = OUTCOME_V2_NAMESPACE;


class HookManager;

class WindowList : public HookListener
{
public:
    static outcome::checked<std::shared_ptr<WindowList>, WindowsError> create(
        HookManager &hm);

    WindowList(WindowList &&) = delete;
    WindowList &operator=(WindowList &&) = delete;
    WindowList(const WindowList &) = delete;
    WindowList &operator=(const WindowList &) = delete;

    void handle_events();

private:
    static BOOL CALLBACK window_enum_proc(HWND, LPARAM);

    WindowList() {}

    void add_window(HWND hwnd);
    void remove_window(HWND hwnd);

    std::map<HWND, Window> hwnd_map_;
    std::optional<outcome::outcome<void, WindowsError>> enum_outcome_;
};
