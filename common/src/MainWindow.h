#pragma once

#include <functional>
#include <windows.h>
#include <outcome.hpp>
#include <memory>
#include "WindowsError.h"

namespace outcome = OUTCOME_V2_NAMESPACE;

class MainWindow
{
public:
    typedef std::function<LRESULT(HWND, UINT, WPARAM, LPARAM)> WindowProc;

    static outcome::checked<std::shared_ptr<MainWindow>, WindowsError> create(
        HINSTANCE hInstance,
        const WCHAR *name,
        WindowProc proc = DefWindowProcW);

    ~MainWindow();

    MainWindow(MainWindow &&) = delete;
    MainWindow &operator=(MainWindow &&) = delete;
    MainWindow(const MainWindow &) = delete;
    MainWindow &operator=(const MainWindow &) = delete;

    outcome::checked<int, WindowsError> run_event_loop();
    void close();

    HWND hwnd() const
    {
        return hwnd_;
    }

private:
    MainWindow();

    static LRESULT CALLBACK proc_bootstrap(HWND hwnd,
                                           UINT uMsg,
                                           WPARAM wParam,
                                           LPARAM lParam);

    HINSTANCE hInstance_;
    ATOM wndclass_;
    HWND hwnd_;
    WindowProc proc_;
};
