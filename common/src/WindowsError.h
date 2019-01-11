#pragma once

#include <windows.h>
#include <string>

class WindowsError
{
public:
    WindowsError();
    explicit WindowsError(DWORD code);

    DWORD code() const;
    std::wstring string() const;

    static WindowsError get_last() { return WindowsError(GetLastError()); };

private:
    DWORD code_;
};
