#include "win32.h"

#include <exception>
#include <list>
#include <shlwapi.h>
#include <tlhelp32.h>

namespace win32
{
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

    outcome::checked<std::wstring, WindowsError> string_to_wide(const std::string &source)
    {
        if (source.empty()) {
            return std::wstring();
        }
        int size = MultiByteToWideChar(
            CP_UTF8,
            MB_PRECOMPOSED | MB_ERR_INVALID_CHARS,
            source.c_str(),
            static_cast<int>(source.length()),
            nullptr,
            0);
        if (size == 0) {
            return WindowsError::get_last();
        }
        wchar_t *buf = new wchar_t[size];
        int written = MultiByteToWideChar(
            CP_UTF8,
            MB_COMPOSITE,
            source.c_str(),
            static_cast<int>(source.length()),
            buf,
            size);
        if (written < size) {
            delete [] buf;
            return WindowsError::get_last();
        }
        std::wstring result(buf, size);
        delete [] buf;
        return result;
    }

    outcome::checked<std::string, WindowsError> string_from_wide(const std::wstring &source)
    {
        if (source.empty()) {
            return std::string();
        }
        int size = WideCharToMultiByte(
            CP_UTF8,
            WC_ERR_INVALID_CHARS,
            source.c_str(),
            static_cast<int>(source.length()),
            nullptr,
            0,
            nullptr,
            nullptr);
        if (size == 0) {
            return WindowsError::get_last();
        }
        char *buf = new char[size];
        int written = WideCharToMultiByte(
            CP_UTF8,
            WC_ERR_INVALID_CHARS,
            source.c_str(),
            static_cast<int>(source.length()),
            buf,
            size,
            nullptr,
            nullptr);
        if (written < size) {
            delete [] buf;
            return WindowsError::get_last();
        }
        std::string result(buf, size);
        delete [] buf;
        return result;
    }
}
