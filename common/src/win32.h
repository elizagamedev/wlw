#pragma once

#include <exception>
#include <functional>
#include <string>
#include <vector>
#include <windows.h>
#include <outcome.hpp>
#include "WindowsError.h"

namespace outcome = OUTCOME_V2_NAMESPACE;

namespace win32
{
    std::vector<std::wstring> get_args();
    outcome::checked<std::wstring, WindowsError> string_to_wide(const std::string &source);
    outcome::checked<std::string, WindowsError> string_from_wide(const std::wstring &source);
}
