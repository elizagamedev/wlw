#pragma once

#include "HookEvent.h"
#include "HookListener.h"
#include "WindowsHook.h"
#include <array>
#include <atomic>
#include <list>
#include <thread>
#include <memory>

class HookManager
{
public:
    HookManager();
    ~HookManager();

    void register_listener(std::shared_ptr<HookListener> listener);
    void push_event(const HookEvent *e);

private:
    static BOOL CALLBACK enum_windows_stop_processes_proc(HWND, LPARAM);
    static const WCHAR *const process_names[2];
    std::array<HANDLE, 2> hook_processes_;
    std::thread process_monitor_thread_;
    std::atomic_bool process_monitor_thread_stop_;
    std::list<std::shared_ptr<HookListener>> listeners_;
};
