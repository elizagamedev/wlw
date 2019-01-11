#include "WindowsError.h"
#include <exception>

// Invalid state
WindowsError::WindowsError()
    : code_(ERROR_SUCCESS)
{
}

WindowsError::WindowsError(DWORD code)
    : code_(code)
{
    if (code_ == ERROR_SUCCESS) {
        throw std::logic_error("Attempted to create a WindowsError of ERROR_SUCCESS");
    }
}

DWORD WindowsError::code() const
{
    if (code_ == ERROR_SUCCESS) {
        throw std::logic_error("Attempted to use uninitialized WindowsError");
    }
    return code_;
}

std::wstring WindowsError::string() const
{
    if (code_ == ERROR_SUCCESS) {
        throw std::logic_error("Attempted to use uninitialized WindowsError");
    }

    LPWSTR buf;

    DWORD size = FormatMessageW(
        FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM
        | FORMAT_MESSAGE_IGNORE_INSERTS,
        nullptr, code_, MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
        reinterpret_cast<LPWSTR>(&buf), 0, nullptr);

    std::wstring result(buf, size);
    LocalFree(buf);
    return result;
}
