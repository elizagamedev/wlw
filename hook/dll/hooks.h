#pragma once

#include "HookEvent.h"
#include "export.h"
#include "WindowsError.h"
#include <windows.h>
#include <outcome.hpp>

namespace outcome = OUTCOME_V2_NAMESPACE;

namespace hooks
{
    extern EXPORT DWORD daemon_process_id;

    outcome::checked<void, WindowsError> EXPORT connect();
    void EXPORT disconnect();
    bool EXPORT is_connected(int ms);

    LRESULT EXPORT CALLBACK callwndproc_proc(int, WPARAM, LPARAM);
    LRESULT EXPORT CALLBACK cbt_proc(int, WPARAM, LPARAM);
}
