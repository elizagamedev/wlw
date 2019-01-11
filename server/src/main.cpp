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
#include <outcome.hpp>

namespace outcome = OUTCOME_V2_NAMESPACE;

int main_cpp(HINSTANCE hInstance,
             HINSTANCE hPrevInstance,
             LPSTR lpCmdLine,
             int nCmdShow)
{
    boost::log::add_file_log("wlw-server.log",
                             boost::log::keywords::auto_flush = true);

    HookManager hook_manager;
    auto window_list = WindowList::create(hook_manager);

    std::shared_ptr<MainWindow> window;
    auto r = MainWindow::create(hInstance, L"wlw-server", [&hook_manager](
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
    if (r) {
        window = r.value();
    } else {
        BOOST_LOG_TRIVIAL(fatal) << "Could not create main window: " << r.error().string();
        return 1;
    }
    if (auto r = window->run_event_loop()) {
        return r.value();
    } else {
        BOOST_LOG_TRIVIAL(fatal) << "Error running main loop: " << r.error().string();
        return 1;
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
        return 1;
    }
}
