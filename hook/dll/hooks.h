#pragma once

#include "HookEvent.h"
#include "export.h"
#include "interop_message_queue.h"
#include <windows.h>

namespace hooks
{
    extern EXPORT DWORD daemon_process_id;

    void EXPORT connect();
    void EXPORT disconnect();
    bool EXPORT is_connected(int ms);

    LRESULT EXPORT CALLBACK callwndproc_proc(int, WPARAM, LPARAM);
    LRESULT EXPORT CALLBACK cbt_proc(int, WPARAM, LPARAM);
}
