#pragma once

#include <functional>
#include <windows.h>

class WindowsHook
{
public:
    WindowsHook(int idHook, HOOKPROC lpfn, HINSTANCE hmod, DWORD dwThreadId);
    ~WindowsHook();

    WindowsHook(WindowsHook &&);
    WindowsHook &operator=(WindowsHook &&);
    WindowsHook(const WindowsHook &) = delete;
    WindowsHook &operator=(const WindowsHook &) = delete;

private:
    HHOOK hook_;
};
