#pragma once

#include "HookEvent.h"


class HookManager;

class HookListener
{
    friend HookManager;

protected:
    template<typename Functor> size_t consume_all_hook_events(Functor &functor)
    {
        return hook_event_queue.consume_all(functor);
    }

private:
    hooks::HookEventQueue hook_event_queue;
};
