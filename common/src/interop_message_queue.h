#pragma once

#include <boost/interprocess/ipc/message_queue.hpp>

typedef boost::interprocess::
    message_queue_t<boost::interprocess::offset_ptr<void, int32_t, uint64_t>>
        interop_message_queue;
