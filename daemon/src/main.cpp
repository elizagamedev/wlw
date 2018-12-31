#include <exception>
#include <windows.h>
#include <psapi.h>
#include <iostream>
#include "win32.h"
// #include "WindowList.h"
// #include "HookManager.h"
#include "HookEvent.h"
#include "MainWindow.h"
#include <boost/log/trivial.hpp>
#include <boost/log/utility/setup/file.hpp>


int main_cpp(HINSTANCE hInstance,
             HINSTANCE hPrevInstance,
             LPSTR lpCmdLine,
             int nCmdShow)
{
    boost::log::add_file_log("wtwm-daemon.log", boost::log::keywords::auto_flush=true);

    // HookManager hook_manager(GetModuleHandleW(L"wtwm.dll"));
    // WindowList window_list(hook_manager);

    MainWindow window(
        hInstance,
        L"wtwm-daemon",
        [](HWND hwnd, UINT uMsg, WPARAM wParam, LPARAM lParam) -> LRESULT {
            switch (uMsg) {
            case WM_COPYDATA:
                BOOST_LOG_TRIVIAL(trace) << "Got a copydata!";
                return 0;
            default:
                return DefWindowProcW(hwnd, uMsg, wParam, lParam);
            }
        });
    return window.run_event_loop();
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
        return 1;
    }
}
