#pragma once

#include "WindowsHook.h"
#include "HookListener.h"
#include <list>
#include <mutex>
#include <thread>
#include <chrono>
#include <atomic>


class HookManager
{
public:
    HookManager(HINSTANCE dll_instance);
    ~HookManager();

    void register_listener(HookListener *listener);

private:
    WindowsHook cbt_hook_;
    std::list<HookListener*> listeners_;
    std::mutex listeners_mutex_;
    std::thread listen_thread_;
    std::atomic_bool listeners_quit_;
};
