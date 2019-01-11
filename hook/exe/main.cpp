#include "MainWindow.h"
#include "WindowsHook.h"
#include "hooks.h"
#include "win32.h"
#include <atomic>
#include <boost/log/trivial.hpp>
#include <boost/log/utility/setup/file.hpp>
#include <exception>
#include <thread>
#include <windows.h>

#ifdef _WIN64
#define BASENAME "wlw_hook64"
#else
#define BASENAME "wlw_hook32"
#endif

class DaemonProcMonitorThread
{
public:
    DaemonProcMonitorThread(std::shared_ptr<MainWindow> window)
        : stop_(false)
    {
        thread_ = std::thread([this, &window]() {
            while (!stop_) {
                if (!hooks::is_connected(1000)) {
                    BOOST_LOG_TRIVIAL(fatal) << "WLW server process is down";
                    window->close();
                    break;
                }
            }
        });
    }

    ~DaemonProcMonitorThread()
    {
        stop_ = true;
        thread_.join();
    }

private:
    std::thread thread_;
    std::atomic_bool stop_;
};

int main_cpp(HINSTANCE hInstance,
             HINSTANCE hPrevInstance,
             LPSTR lpCmdLine,
             int nCmdShow)
{
    boost::log::add_file_log(BASENAME ".log",
                             boost::log::keywords::auto_flush = true);

    std::vector<std::wstring> args = win32::get_args();
    if (args.size() < 2) {
        BOOST_LOG_TRIVIAL(fatal) << "No PID given";
        return 1;
    }
    hooks::daemon_process_id = std::stoi(args[1]);
    BOOST_LOG_TRIVIAL(trace) << "Before connect";
    hooks::connect();

    HINSTANCE dll_instance = GetModuleHandleW(BASENAME L".dll");
    WindowsHook cwp_hook(WH_CALLWNDPROC, hooks::callwndproc_proc, dll_instance,
                         0);
    WindowsHook cbt_hook(WH_CBT, hooks::cbt_proc, dll_instance, 0);

    std::shared_ptr<MainWindow> window;
    if (auto r = MainWindow::create(hInstance, L"wlw-hook")) {
        window = r.value();
    } else {
        BOOST_LOG_TRIVIAL(fatal) << "Could not create main window: " << r.error().string();
    }
    DaemonProcMonitorThread daemon_monitor_thread(window);
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
