#include "HookEvent.h"
#include "HookManager.h"
#include "MainWindow.h"
#include "WindowList.h"
#include "win32.h"
#include <boost/log/trivial.hpp>
#include <boost/log/utility/setup/file.hpp>
#include <exception>
#include <iostream>
#include <psapi.h>
#include <windows.h>

int main_cpp(HINSTANCE hInstance,
             HINSTANCE hPrevInstance,
             LPSTR lpCmdLine,
             int nCmdShow)
{
    boost::log::add_file_log("wlw-server.log",
                             boost::log::keywords::auto_flush = true);

    HookManager hook_manager;
    WindowList window_list(hook_manager);

    MainWindow window(hInstance, L"wlw-server", [&hook_manager](
                                                    HWND hwnd, UINT uMsg,
                                                    WPARAM wParam,
                                                    LPARAM lParam) -> LRESULT {
        switch (uMsg) {
        case WM_COPYDATA: {
            const COPYDATASTRUCT *cds
                = reinterpret_cast<const COPYDATASTRUCT *>(lParam);
            if (cds->dwData == 0xDEADBEEF && cds->cbData == sizeof(HookEvent)) {
                hook_manager.push_event(
                    reinterpret_cast<HookEvent *>(cds->lpData));
            }
            return TRUE;
        }
        default:
            return DefWindowProcW(hwnd, uMsg, wParam, lParam);
        }
    });
    BOOST_LOG_TRIVIAL(trace) << "Server window handle: " << window.hwnd();
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
