#pragma once

#include <functional>
#include <windows.h>

class MainWindow
{
    friend void swap(MainWindow &lhs, MainWindow &rhs);

public:
    typedef std::function<LRESULT(HWND, UINT, WPARAM, LPARAM)> WindowProc;

    MainWindow(HINSTANCE hInstance,
               const WCHAR *name,
               WindowProc proc = DefWindowProcW);
    ~MainWindow();

    MainWindow(MainWindow &&);
    MainWindow &operator=(MainWindow &&);
    MainWindow(const MainWindow &) = delete;
    MainWindow &operator=(const MainWindow &) = delete;

    int run_event_loop();
    void close();

    HWND hwnd() const
    {
        return hwnd_;
    }

private:
    static LRESULT CALLBACK proc_bootstrap(HWND hwnd,
                                           UINT uMsg,
                                           WPARAM wParam,
                                           LPARAM lParam);

    HINSTANCE hInstance_;
    ATOM wndclass_;
    HWND hwnd_;
    WindowProc proc_;
};

void swap(MainWindow &lhs, MainWindow &rhs);
