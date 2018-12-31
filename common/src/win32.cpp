#include "win32.h"

#include <exception>
#include <list>
#include <shlwapi.h>
#include <tlhelp32.h>


namespace win32
{
    std::string get_last_error_string()
    {
        LPVOID lpMsgBuf;
        DWORD dw = GetLastError();

        DWORD size = FormatMessageA(
            FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM
                | FORMAT_MESSAGE_IGNORE_INSERTS,
            NULL, dw, MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
            (LPTSTR)&lpMsgBuf, 0, NULL);

        std::string result(static_cast<char *>(lpMsgBuf), size);
        LocalFree(lpMsgBuf);
        return result;
    }

    std::runtime_error get_last_error_exception()
    {
        return std::runtime_error(get_last_error_string());
    }

    std::vector<std::wstring> get_args()
    {
        int argc;
        LPWSTR *argv = CommandLineToArgvW(GetCommandLineW(), &argc);
        std::vector<std::wstring> args;
        args.reserve(argc);
        for (int i = 0; i < argc; ++i) {
            args.push_back(std::wstring(argv[i]));
        }
        LocalFree(argv);
        return args;
    }

    std::wstring get_module_file_name(HMODULE module)
    {
        WCHAR *buffer;
        DWORD size = 16;
        for (;;) {
            buffer = new WCHAR[size];
            size = GetModuleFileNameW(module, buffer, size);
            DWORD error = GetLastError();
            if (error == ERROR_SUCCESS) {
                break;
            }
            delete[] buffer;
            if (error == ERROR_INSUFFICIENT_BUFFER) {
                size *= 2;
            } else {
                // Weird error that can't be handled
                throw get_last_error_exception();
            }
        }
        std::wstring result(buffer, size);
        delete[] buffer;
        return result;
    }

    std::wstring get_system_directory()
    {
        UINT size = GetSystemDirectoryW(nullptr, 0);
        if (size == 0) {
            throw win32::get_last_error_exception();
        }
        WCHAR *buf = new WCHAR[size];
        if (GetSystemDirectoryW(buf, size) != size - 1) {
            throw win32::get_last_error_exception();
        }
        std::wstring result(buf, size - 1);
        delete[] buf;
        return result;
    }
}
