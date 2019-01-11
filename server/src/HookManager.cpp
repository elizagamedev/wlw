#include "HookManager.h"
#include <boost/dll.hpp>
#include <boost/filesystem.hpp>
#include <boost/log/trivial.hpp>
#include <chrono>
#include <sstream>
#include <outcome.hpp>
#include <algorithm>

namespace outcome = OUTCOME_V2_NAMESPACE;

static outcome::checked<HANDLE, WindowsError> start_hook_process(const WCHAR *name)
{
    boost::filesystem::path bin_dir
        = boost::dll::program_location().parent_path();
    boost::filesystem::path exe_path = bin_dir / name;

    std::wstringstream command_stream;
    command_stream << exe_path << L" " << GetCurrentProcessId();
    std::wstring command_const = command_stream.str();
    std::vector<WCHAR> command(command_const.begin(), command_const.end());
    command.push_back(L'\0');

    BOOST_LOG_TRIVIAL(trace) << "Starting hook process: " << command_const;

    // Start hook clients
    PROCESS_INFORMATION pi = {};
    STARTUPINFOW si = {};
    si.cb = sizeof(si);
    BOOL result = CreateProcessW(nullptr,        // lpApplicationName
                                 command.data(), // lpCommandLine
                                 nullptr,        // lpProcessAttributes
                                 nullptr,        // lpThreadAttributes
                                 FALSE,          // bInheritHandles
                                 0,              // dwCreationFlags
                                 nullptr,        // lpEnvironment
                                 bin_dir.native().c_str(), // lpCurrentDirectory
                                 &si, &pi);

    if (!result) {
        return WindowsError::get_last();
    }

    CloseHandle(pi.hThread);
    return pi.hProcess;
}

HookManager::HookManager()
    : hook_processes_({})
    , process_monitor_thread_stop_(false)
{
    process_monitor_thread_ = std::thread([this]() {
        while (!process_monitor_thread_stop_) {
            for (size_t i = 0; i < hook_processes_.size(); ++i) {
                bool start_process = false;
                if (hook_processes_[i] == NULL) {
                    start_process = true;
                } else if (WaitForSingleObject(hook_processes_[i], 2000)
                           != WAIT_TIMEOUT) {
                    CloseHandle(hook_processes_[i]);
                    hook_processes_[i] = NULL;
                    start_process = true;
                }
                if (start_process) {
                    if (auto r = start_hook_process(process_names[i])) {
                        hook_processes_[i] = r.value();
                    } else {
                        BOOST_LOG_TRIVIAL(error)
                        << L"Error starting hook process: "
                        << r.error().string();
                    }
                }
            }
            std::this_thread::sleep_for(std::chrono::seconds(1));
        }
    });
}

HookManager::~HookManager()
{
    // Discover each hook process' window handles and send a nice close
    // message. If that doesn't work, force terminate.
    process_monitor_thread_stop_ = true;
    process_monitor_thread_.join();
    EnumWindows(&HookManager::enum_windows_stop_processes_proc,
                reinterpret_cast<LPARAM>(this));
    // Give the processes ample time to close before terminating them forcibly.
    for (size_t i = 0; i < hook_processes_.size(); ++i) {
        if (hook_processes_[i] != NULL) {
            if (WaitForSingleObject(hook_processes_[i], 8000) == WAIT_TIMEOUT) {
                TerminateProcess(hook_processes_[i], 1);
            }
            CloseHandle(hook_processes_[i]);
        }
    }
}

void HookManager::register_listener(std::shared_ptr<HookListener> listener)
{
    BOOST_LOG_TRIVIAL(trace) << "HookManager registering new listener "
                             << listener;
    listeners_.push_back(std::move(listener));
}

void HookManager::push_event(const HookEvent *e)
{
    for (auto &listener : listeners_) {
        listener->hook_event_queue.push(HookEvent(*e));
    }
}

BOOL CALLBACK HookManager::enum_windows_stop_processes_proc(HWND hwnd,
                                                            LPARAM lParam)
{
    HookManager *hm = reinterpret_cast<HookManager *>(lParam);
    DWORD process_id;
    GetWindowThreadProcessId(hwnd, &process_id);
    for (size_t i = 0; i < hm->hook_processes_.size(); ++i) {
        if (process_id == GetProcessId(hm->hook_processes_[i])) {
            SendMessageW(hwnd, WM_CLOSE, 0, 0);
            break;
        }
    }
    return TRUE;
}

const WCHAR *const HookManager::process_names[2] = {
    L"wlw_hook32.exe", L"wlw_hook64.exe",
};
