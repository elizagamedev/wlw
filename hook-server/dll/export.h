#pragma once

#ifdef WTWM_HOOK_SERVER_DLL
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __declspec(dllimport)
#endif
