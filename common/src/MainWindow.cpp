#include "MainWindow.h"
#include "win32.h"
#include <boost/log/trivial.hpp>


MainWindow::MainWindow(HINSTANCE hInstance, const WCHAR *name, WindowProc proc)
    : hInstance_(hInstance)
    , wndclass_(NULL)
    , hwnd_(NULL)
    , proc_(std::move(proc))
{
    WNDCLASSEXW class_opts = {};
    class_opts.cbSize = sizeof(class_opts);
    class_opts.lpfnWndProc = &MainWindow::proc_bootstrap;
    class_opts.hInstance = hInstance_;
    class_opts.lpszClassName = name;
    wndclass_ = RegisterClassExW(&class_opts);
    if (wndclass_ == NULL) {
        throw win32::get_last_error_exception();
    }

    hwnd_ = CreateWindowExW(
        0, // dwExStyle
        reinterpret_cast<LPWSTR>(wndclass_), // lpClassName
        name, // lpWindowName
        0, // dwStyle
        0, 0, 0, 0, // x, y, nWidth, nHeight
        NULL, // hWndParent
        NULL, // hMenu
        hInstance_, // hInstance
        nullptr // lpParam
        );
    if (hwnd_ == NULL) {
        throw win32::get_last_error_exception();
    }

    SetWindowLongPtrW(hwnd_, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(this));
}

MainWindow::MainWindow(MainWindow &&o)
    : hInstance_(NULL)
    , wndclass_(NULL)
    , hwnd_(NULL)
    , proc_(DefWindowProcW)
{
    using std::swap;
    swap(*this, o);
}

MainWindow &MainWindow::operator=(MainWindow &&o)
{
    using std::swap;
    swap(*this, o);
    return *this;
}

MainWindow::~MainWindow()
{
    if (hwnd_ != NULL) {
        DestroyWindow(hwnd_);
    }
    if (wndclass_ != NULL) {
        UnregisterClassW(reinterpret_cast<LPWSTR>(wndclass_), hInstance_);
    }
}

int MainWindow::run_event_loop()
{
    BOOST_LOG_TRIVIAL(trace) << "Running Windows event loop";
    for (;;) {
        MSG msg;
        BOOL bRet = GetMessageW(&msg, NULL, 0, 0);

        if (bRet > 0) {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        } else if (bRet < 0) {
            throw win32::get_last_error_exception();
        } else {
            return static_cast<int>(msg.wParam);
        }
    }
}

void MainWindow::close()
{
    PostMessageW(hwnd_, WM_CLOSE, 0, 0);
}

LRESULT CALLBACK MainWindow::proc_bootstrap(
    HWND hwnd,
    UINT uMsg,
    WPARAM wParam,
    LPARAM lParam)
{
    MainWindow *self = reinterpret_cast<MainWindow*>(
        GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    // Handle close/destroy here
    switch (uMsg) {
    case WM_CLOSE:
    case WM_DESTROY:
        BOOST_LOG_TRIVIAL(trace) << "Got a close/destroy message";
        PostQuitMessage(0);
        return 0;
    }
    if (self == nullptr) {
        return DefWindowProcW(hwnd, uMsg, wParam, lParam);
    } else {
        return self->proc_(hwnd, uMsg, wParam, lParam);
    }
}

void swap(MainWindow &lhs, MainWindow &rhs)
{
    using std::swap;
    swap(lhs.hInstance_, rhs.hInstance_);
    swap(lhs.wndclass_, rhs.wndclass_);
    swap(lhs.hwnd_, rhs.hwnd_);
    swap(lhs.proc_, rhs.proc_);
}
