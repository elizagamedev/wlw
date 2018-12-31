#include "hooks.h"
#include "win32.h"
#include <boost/log/trivial.hpp>

static BOOL CALLBACK enum_windows_proc(HWND hwnd, LPARAM lParam)
{
    DWORD process_id;
    GetWindowThreadProcessId(hwnd, &process_id);
    if (process_id == hooks::daemon_process_id) {
        hooks::daemon_window = hwnd;
        return FALSE;
    }
    return TRUE;
}

namespace hooks
{
#pragma data_seg(".shared")
    DWORD daemon_process_id = 0;
#pragma data_seg()
#pragma comment(linker, "/section:.shared,RWS")

    HANDLE daemon_process_handle = NULL;
    HWND daemon_window = NULL;

    void connect()
    {
        if (daemon_process_id == 0) {
            throw std::runtime_error("Process ID is 0");
        }
        // Find the daemon's window and open its process handle.
        if (!EnumWindows(enum_windows_proc, 0)) {
            throw win32::get_last_error_exception();
        }
        daemon_process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,
                                            FALSE,
                                            daemon_process_id);
    }

    void disconnect()
    {
        CloseHandle(daemon_process_handle);
        daemon_process_handle = NULL;
    }

    bool is_connected()
    {
        if (daemon_process_id == 0) {
            return false;
        }
        if (daemon_process_handle == NULL) {
            return false;
        }
        return WaitForSingleObject(daemon_process_handle, 0) == WAIT_TIMEOUT;
    }

    LRESULT CALLBACK cbt_proc(int nCode, WPARAM wParam, LPARAM lParam)
    {
        BOOST_LOG_TRIVIAL(trace) << "cbt proc";
        // HookEvent event = {WH_CBT, nCode, wParam, lParam};

        // message_queue->try_send(&event, sizeof(event), 0);
        return CallNextHookEx(NULL, nCode, wParam, lParam);
    }
}
