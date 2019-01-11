#include "WindowsHook.h"

outcome::checked<WindowsHook, WindowsError> WindowsHook::create(
    int idHook, HOOKPROC lpfn, HINSTANCE hmod, DWORD dwThreadId)
{
    WindowsHook result;
    result.hook_ = SetWindowsHookExW(idHook, lpfn, hmod, dwThreadId);
    if (result.hook_ == NULL) {
        return WindowsError::get_last();
    }
    return result;
}

WindowsHook::WindowsHook(WindowsHook &&o)
{
    swap(*this, o);
}

WindowsHook &WindowsHook::operator=(WindowsHook &&o)
{
    swap(*this, o);
    return *this;
}

WindowsHook::~WindowsHook()
{
    if (hook_ != NULL) {
        UnhookWindowsHookEx(hook_);
    }
}

void swap(WindowsHook &lhs, WindowsHook &rhs)
{
    using std::swap;
    swap(lhs.hook_, rhs.hook_);
}
