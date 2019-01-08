#pragma once

#include "HookEvent.h"
#include <boost/lockfree/queue.hpp>

class HookManager;

class HookListener
{
    friend HookManager;
    typedef boost::lockfree::queue<HookEvent, boost::lockfree::capacity<512>>
        HookEventQueue;

protected:
    template <typename Functor>
    size_t consume_all_hook_events(Functor &functor)
    {
        return hook_event_queue.consume_all(functor);
    }

private:
    HookEventQueue hook_event_queue;
};
