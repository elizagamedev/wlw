#include "MainWindow.h"

outcome::checked<std::shared_ptr<MainWindow>, WindowsError> MainWindow::create(
    HINSTANCE hInstance, const WCHAR *name, WindowProc proc)
{
    std::shared_ptr<MainWindow> result(new MainWindow());
    result->hInstance_ = hInstance;
    result->proc_ = std::move(proc);

    WNDCLASSEXW class_opts = {};
    class_opts.cbSize = sizeof(class_opts);
    class_opts.lpfnWndProc = &proc_bootstrap;
    class_opts.hInstance = hInstance;
    class_opts.lpszClassName = name;
    result->wndclass_ = RegisterClassExW(&class_opts);
    if (result->wndclass_ == NULL) {
        return WindowsError::get_last();
    }

    result->hwnd_ = CreateWindowExW(
        0,                                   // dwExStyle
        reinterpret_cast<LPWSTR>(result->wndclass_), // lpClassName
        name,                                // lpWindowName
        0,                                   // dwStyle
        0, 0, 0, 0, // x, y, nWidth, nHeight
        NULL,       // hWndParent
        NULL,       // hMenu
        hInstance,  // hInstance
        nullptr     // lpParam
        );
    if (result->hwnd_ == NULL) {
        return WindowsError::get_last();
    }

    SetWindowLongPtrW(result->hwnd_, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(result.get()));
    return result;
}

MainWindow::MainWindow()
    : hInstance_(NULL)
    , wndclass_(NULL)
    , hwnd_(NULL)
    , proc_(DefWindowProcW)
{
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

outcome::checked<int, WindowsError> MainWindow::run_event_loop()
{
    for (;;) {
        MSG msg;
        BOOL bRet = GetMessageW(&msg, NULL, 0, 0);

        if (bRet > 0) {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        } else if (bRet < 0) {
            return WindowsError::get_last();
        } else {
            return static_cast<int>(msg.wParam);
        }
    }
}

void MainWindow::close()
{
    PostMessageW(hwnd_, WM_CLOSE, 0, 0);
}

LRESULT CALLBACK MainWindow::proc_bootstrap(HWND hwnd,
                                            UINT uMsg,
                                            WPARAM wParam,
                                            LPARAM lParam)
{
    MainWindow *self = reinterpret_cast<MainWindow *>(
        GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    // Handle close/destroy here
    switch (uMsg) {
    case WM_CLOSE:
        PostQuitMessage(0);
        return 0;
    }
    if (self == nullptr) {
        return DefWindowProcW(hwnd, uMsg, wParam, lParam);
    } else {
        return self->proc_(hwnd, uMsg, wParam, lParam);
    }
}
