#pragma once

#include <windows.h>
#include "HookEvent.h"
#include "export.h"
#include "interop_message_queue.h"


namespace hooks
{
    extern EXPORT DWORD daemon_process_id;

    void EXPORT connect();
    void EXPORT disconnect();
    bool EXPORT is_connected(int ms);

    LRESULT EXPORT CALLBACK cbt_proc(int, WPARAM, LPARAM);
}
