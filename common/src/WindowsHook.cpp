#include "WindowsHook.h"
#include "win32.h"
#include <boost/log/trivial.hpp>

WindowsHook::WindowsHook(int idHook,
                         HOOKPROC lpfn,
                         HINSTANCE hmod,
                         DWORD dwThreadId)
{
    if ((hook_ = SetWindowsHookExW(idHook, lpfn, hmod, dwThreadId)) == NULL) {
        throw win32::get_last_error_exception();
    }
    BOOST_LOG_TRIVIAL(trace) << "Registered new hook (idHook " << idHook << ")";
}

WindowsHook::WindowsHook(WindowsHook &&o)
{
    using std::swap;
    swap(hook_, o.hook_);
}

WindowsHook &WindowsHook::operator=(WindowsHook &&o)
{
    using std::swap;
    swap(hook_, o.hook_);
    return *this;
}

WindowsHook::~WindowsHook()
{
    if (hook_ != NULL) {
        UnhookWindowsHookEx(hook_);
        hook_ = NULL;
    }
}
