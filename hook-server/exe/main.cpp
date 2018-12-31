#include "WindowsHook.h"
#include "hooks.h"
#include "win32.h"
#include "MainWindow.h"
#include <boost/log/trivial.hpp>
#include <boost/log/utility/setup/file.hpp>
#include <exception>
#include <windows.h>
#include <thread>
#include <atomic>


#ifdef _WIN64
#define DLL_NAME L"wtwm_hook_server64.dll"
#else
#define DLL_NAME L"wtwm_hook_server32.dll"
#endif


class DaemonProcMonitorThread
{
public:
    DaemonProcMonitorThread(MainWindow &window)
        : stop_(false)
    {
        thread_ = std::thread(
            [this, &window]() {
                while (!stop_) {
                    if (!hooks::is_connected(1000)) {
                        BOOST_LOG_TRIVIAL(fatal) << "WTWM daemon process is down";
                        window.close();
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
    boost::log::add_file_log("wtwm-hook-server.log", boost::log::keywords::auto_flush=true);

    std::vector<std::wstring> args = win32::get_args();
    if (args.size() < 2) {
        BOOST_LOG_TRIVIAL(fatal) << "No PID given";
        return 1;
    }
    hooks::daemon_process_id = std::stoi(args[1]);
    hooks::connect();

    HINSTANCE dll_instance = GetModuleHandleW(DLL_NAME);
    WindowsHook cbt_hook(WH_CBT, hooks::cbt_proc, dll_instance, 0);

    MainWindow window(hInstance, L"wtwm-hook-server");
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
