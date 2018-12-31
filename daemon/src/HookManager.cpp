#include "HookManager.h"
#include "hooks.h"
#include <boost/log/trivial.hpp>


HookManager::HookManager(HINSTANCE dll_instance)
    : cbt_hook_(WH_CBT, hooks::cbt_proc, dll_instance, 0)
    , listeners_quit_(false)
{
    listen_thread_ = std::thread(
        [this]() {
            BOOST_LOG_TRIVIAL(trace) << "HookManager listening to event queue";
            for (;;) {
                // Consume all events into listeners' queues
                hooks::event_queue.consume_all(
                    [this](HookEvent &event) {
                        BOOST_LOG_TRIVIAL(trace) << "HookManager received " << event.idHook << " event";
                        std::lock_guard<std::mutex> lock(listeners_mutex_);
                        for (HookListener *listener : listeners_) {
                            listener->hook_event_queue.push(event);
                        }
                    });
                std::this_thread::sleep_for(std::chrono::milliseconds(100));
                // Quit if done
                if (listeners_quit_) {
                    break;
                }
            }
        });
}

HookManager::~HookManager()
{
    listeners_quit_ = true;
    listen_thread_.join();
}

void HookManager::register_listener(HookListener *listener)
{
    BOOST_LOG_TRIVIAL(trace) << "HookManager registering new listener " << listener;
    std::lock_guard<std::mutex> lock(listeners_mutex_);
    listeners_.push_back(listener);
}
