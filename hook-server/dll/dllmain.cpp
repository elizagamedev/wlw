#include <windows.h>
#include <memory>
#include "hooks.h"
#include "HookEvent.h"
#include <boost/interprocess/exceptions.hpp>



extern "C" BOOL WINAPI DllMain(HINSTANCE hinstDLL,
                               DWORD fdwReason,
                               LPVOID lpvReserved)
{
    switch (fdwReason) {
    case DLL_PROCESS_ATTACH:
        DisableThreadLibraryCalls(hinstDLL);
        // If the daemon process ID is 0, that means we are loaded in the
        // server exe and have not set it yet.
        if (daemon_process_id == 0) {
            return TRUE;
        }
        try {
            hooks::connect();
        } catch (const std::exception &) {
            return FALSE;
        }
        return TRUE;
    case DLL_PROCESS_DETACH:
        hooks::disconnect();
        return TRUE;
    }
    return FALSE;
}
