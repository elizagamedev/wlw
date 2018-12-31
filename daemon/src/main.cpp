#include <exception>
#include <windows.h>
#include <psapi.h>
#include <iostream>
#include "win32.h"
// #include "WindowList.h"
// #include "HookManager.h"
#include "HookEvent.h"
#include <boost/log/trivial.hpp>
#include <boost/interprocess/ipc/message_queue.hpp>


typedef boost::interprocess::message_queue_t<
    boost::interprocess::offset_ptr<void, int32_t, uint64_t>> interop_message_queue;

static HWND window = NULL;

LRESULT CALLBACK window_proc(HWND hwnd,
                             UINT uMsg,
                             WPARAM wParam,
                             LPARAM lParam)
{
    switch (uMsg) {
    case WM_CLOSE:
    case WM_DESTROY:
        BOOST_LOG_TRIVIAL(trace) << "Got a close/destroy message";
        PostQuitMessage(0);
        return 0;
    default:
        return DefWindowProcW(hwnd, uMsg, wParam, lParam);
    }
}

int main_cpp(HINSTANCE hInstance,
             HINSTANCE hPrevInstance,
             LPSTR lpCmdLine,
             int nCmdShow)
{
    // HookManager hook_manager(GetModuleHandleW(L"wtwm.dll"));
    // WindowList window_list(hook_manager);

    boost::interprocess::message_queue::remove("wtwm-hooks");
    interop_message_queue mq(
        boost::interprocess::create_only, "wtwm-hooks", 1024, sizeof(HookEvent));


    for (;;) {
        HookEvent e;
        boost::interprocess::message_queue::size_type received_size;
        unsigned int priority;
        mq.receive(&e, sizeof(e), received_size, priority);
        if (received_size != sizeof(e)) {
            BOOST_LOG_TRIVIAL(error) << "Size incorrect";
        }
        BOOST_LOG_TRIVIAL(trace) << "Got a message with idHook = " << e.idHook;
    }

    return 0;

    BOOST_LOG_TRIVIAL(trace) << "Running Windows event loop";

    // We create a Window to allow for graceful termination from other
    // processes/console.
    WNDCLASSEXW class_opts = {};
    class_opts.cbSize = sizeof(class_opts);
    class_opts.lpfnWndProc = window_proc;
    class_opts.hInstance = hInstance;
    class_opts.lpszClassName = L"wtwm";
    ATOM wndclass = RegisterClassExW(&class_opts);

    window = CreateWindowExW(
        0, // dwExStyle
        reinterpret_cast<LPWSTR>(wndclass), // lpClassName
        L"WTWM", // lpWindowName
        0, // dwStyle
        0, 0, 0, 0, // x, y, nWidth, nHeight
        NULL, // hWndParent
        NULL, // hMenu
        hInstance, // hInstance
        nullptr // lpParam
        );

    for (;;) {
        MSG msg;
        BOOL bRet = GetMessageW(&msg, NULL, 0, 0);

        if (bRet > 0) {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        } else if (bRet < 0) {
            throw win32::get_last_error_exception();
        } else {
            return msg.wParam;
        }
    }
}

extern "C" int CALLBACK WinMain(HINSTANCE hInstance,
                                HINSTANCE hPrevInstance,
                                LPSTR lpCmdLine,
                                int nCmdShow)
{
    try {
        return main_cpp(hInstance, hPrevInstance, lpCmdLine, nCmdShow);
    } catch (const std::exception &e) {
        BOOST_LOG_TRIVIAL(fatal) << e.what();
        MessageBoxA(NULL, e.what(), "WTWM", MB_OK | MB_ICONERROR);
        return 1;
    }
}

#ifndef _WINMAIN_
static BOOL WINAPI console_ctrl_handler(DWORD dwCtrlType)
{
    BOOST_LOG_TRIVIAL(trace) << "Interrupted!";
    if (window != NULL) {
        PostMessage(window, WM_CLOSE, 0, 0);
    }
    return TRUE;
}

extern "C" int main()
{
    SetConsoleCtrlHandler(console_ctrl_handler, TRUE);
    return WinMain(GetModuleHandle(NULL), NULL, GetCommandLineA(), SW_SHOWNORMAL);
}
#endif
