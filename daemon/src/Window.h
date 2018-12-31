#pragma once

#include <string>
#include <optional>
#include <windows.h>


class Window
{
public:
    Window(HWND hwnd);

    const WINDOWINFO &windowinfo() const { return windowinfo_; }
    const std::wstring &title() const { return title_; }
    const std::optional<std::wstring> &process_name() const { return process_name_; }

private:
    HWND hwnd_;
    WINDOWINFO windowinfo_;
    std::wstring title_;
    std::optional<std::wstring> process_name_;
};
