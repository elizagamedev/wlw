#include <exception>
#include <windows.h>
#include <atomic>
#include <thread>
#include <chrono>
#include "win32.h"
#include <boost/log/trivial.hpp>
#include <boost/date_time.hpp>
#include "WindowsHook.h"
#include "hooks.h"


#ifdef _WIN64
#define DLL_NAME L"wtwm_hook_server64.dll"
#else
#define DLL_NAME L"wtwm_hook_server32.dll"
#endif


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
    // Hooks!
    HINSTANCE dll_instance = GetModuleHandleW(DLL_NAME);
    WindowsHook cbt_hook(WH_CBT, hooks::cbt_proc, dll_instance, 0);

    // // Heartbeat thread
    // std::atomic_bool heartbeat_end = false;
    // std::atomic<std::chrono::system_clock::time_point> last_heartbeat =
    //     std::chrono::system_clock::now();

    // // Boost is not fault tolerant at all when it comes to interprocess message
    // // queues. An ugly hack is required to terminate the process in the case
    // // that the daemon is not listening.
    // std::thread heartbeat_thread(
    //     [&heartbeat_end, &last_heartbeat]() {
    //         try {
    //             BOOST_LOG_TRIVIAL(trace) << "Attempting to open a heartbeat connection";
    //             interop_message_queue heartbeat_mq(
    //                 boost::interprocess::open_only, "wtwm-hooks");
    //             BOOST_LOG_TRIVIAL(trace) << "Connected";
    //             for (;;) {
    //                 if (heartbeat_end) {
    //                     break;
    //                 }
    //                 HookEvent event = {-1, 0, 0, 0};
    //                 BOOST_LOG_TRIVIAL(trace) << "Sending a heartbeat message";
    //                 heartbeat_mq.try_send(&event, sizeof(event), 0);
    //                 last_heartbeat = std::chrono::system_clock::now();
    //                 std::this_thread::sleep_for(std::chrono::seconds(1));
    //             }
    //         } catch (boost::interprocess::interprocess_exception &) {
    //             // We failed to connect, so shut down
    //             BOOST_LOG_TRIVIAL(error) << "Heartbeat failed";
    //             PostMessage(window, WM_CLOSE, 0, 0);
    //         }
    //     });

    // std::thread heartbeat_watch_thread(
    //     [&heartbeat_end, &last_heartbeat]() {
    //         for (;;) {
    //             if (heartbeat_end) {
    //                 break;
    //             }
    //             BOOST_LOG_TRIVIAL(trace) << "Monitoring heartbeat...";
    //             if (std::chrono::system_clock::now() - last_heartbeat.load() > std::chrono::seconds(3)) {
    //                 // Too long since last heartbeat. Abort.
    //                 BOOST_LOG_TRIVIAL(error) << "Heartbeat failed. Aborting...";
    //                 PostMessage(window, WM_CLOSE, 0, 0);
    //                 break;
    //             }
    //             std::this_thread::sleep_for(std::chrono::seconds(1));
    //         }
    //     });

    BOOST_LOG_TRIVIAL(trace) << "Running Windows event loop";

    // We create a Window to allow for graceful termination from other
    // processes/console.
    WNDCLASSEXW class_opts = {};
    class_opts.cbSize = sizeof(class_opts);
    class_opts.lpfnWndProc = window_proc;
    class_opts.hInstance = hInstance;
    class_opts.lpszClassName = L"wtwm-hook-server";
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

    int return_value;
    for (;;) {
        MSG msg;
        BOOL bRet = GetMessageW(&msg, NULL, 0, 0);

        if (bRet > 0) {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        } else if (bRet < 0) {
            throw win32::get_last_error_exception();
        } else {
            return_value = msg.wParam;
            break;
        }
    }

    // heartbeat_end = true;
    // heartbeat_thread.join();
    // heartbeat_watch_thread.join();
    return return_value;
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
