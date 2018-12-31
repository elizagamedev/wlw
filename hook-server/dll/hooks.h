#pragma once

#include <windows.h>
#include "HookEvent.h"
#include "export.h"
#include "interop_message_queue.h"


namespace hooks
{
#pragma data_seg(".shared")
    extern EXPORT DWORD daemon_process_id;
#pragma data_seg()
#pragma comment(linker, "/section:.shared,RWS")

    void EXPORT connect();
    void EXPORT disconnect();
    bool EXPORT is_connected();

    LRESULT EXPORT CALLBACK cbt_proc(int, WPARAM, LPARAM);
}
