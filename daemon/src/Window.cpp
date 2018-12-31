#include "Window.h"
#include "win32.h"
#include <psapi.h>
#include <iostream>


Window::Window(HWND hwnd)
    : hwnd_(hwnd)
    , windowinfo_({})
{
    // Window info
    windowinfo_.cbSize = sizeof(windowinfo_);
    if (!GetWindowInfo(hwnd_, &windowinfo_)) {
        throw win32::get_last_error_exception();
    }
    // Title
    {
        int title_length = GetWindowTextLengthW(hwnd_);
        if (title_length > 0) {
            WCHAR *title_buffer = new WCHAR[title_length + 1];
            GetWindowTextW(hwnd_, title_buffer, title_length + 1);
            // According to the docs, GetWindowTextLengthW might return a
            // buffer size larger than the actual title, so don't depend on it
            // to determine the actual length of the string
            title_ = title_buffer;
            delete [] title_buffer;
        }
    }
    // Process name
    {
        DWORD process_id;
        GetWindowThreadProcessId(hwnd, &process_id);
        HANDLE process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, process_id);
        if (process == NULL) {
            // Access is likely denied, so don't populate the process name.
            process_name_ = std::nullopt;
        } else {
            // This function does not provide any way to get its required buffer
            // size, so brute force it.
            WCHAR *buffer;
            DWORD size = 16;
            DWORD copied_size = 0;
            for (;;) {
                buffer = new WCHAR[size];
                copied_size = GetModuleBaseNameW(process, NULL, buffer, size);
                if (copied_size == 0 || copied_size < size) {
                    break;
                }
                delete[] buffer;
                size *= 2;
            }
            if (copied_size > 0) {
                process_name_ = std::wstring(buffer, copied_size);
            }
            delete[] buffer;
            CloseHandle(process);
        }
    }
}
