#pragma once

#include <functional>
#include <windows.h>
#include <outcome.hpp>
#include "WindowsError.h"

namespace outcome = OUTCOME_V2_NAMESPACE;

class WindowsHook
{
    friend void swap(WindowsHook &lhs, WindowsHook &rhs);
public:
    static outcome::checked<WindowsHook, WindowsError> create(
        int idHook, HOOKPROC lpfn, HINSTANCE hmod, DWORD dwThreadId);
    WindowsHook() : hook_(NULL) {}
    ~WindowsHook();

    WindowsHook(WindowsHook &&);
    WindowsHook &operator=(WindowsHook &&);
    WindowsHook(const WindowsHook &) = delete;
    WindowsHook &operator=(const WindowsHook &) = delete;

private:
    HHOOK hook_;
};

void swap(WindowsHook &lhs, WindowsHook &rhs);
