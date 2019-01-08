#pragma once

#include <cstdint>
#include <cstring>
#include <type_traits>
#include <windows.h>

#pragma pack(push, 1)
template <typename NativeType, typename InternalType>
struct PortableType {
    PortableType()
        : value(0)
    {
    }

    PortableType(NativeType value)
    {
        if
            constexpr(std::is_pointer<NativeType>::value)
            {
                this->value = static_cast<InternalType>(
                    reinterpret_cast<intptr_t>(value));
            }
        else {
            this->value = static_cast<InternalType>(value);
        }
    }

    operator NativeType() const
    {
        if
            constexpr(std::is_pointer<NativeType>::value)
            {
                return reinterpret_cast<NativeType>(
                    static_cast<intptr_t>(value));
            }
        else {
            return static_cast<NativeType>(value);
        }
    }

    InternalType value;
};

typedef PortableType<BOOL, int8_t> PortableBOOL;
typedef PortableType<DWORD, uint32_t> PortableDWORD;
typedef PortableType<HWND, uint32_t> PortableHWND;
typedef PortableType<HINSTANCE, uint32_t> PortableHINSTANCE;
typedef PortableType<HMENU, uint32_t> PortableHMENU;
typedef PortableType<LONG, int32_t> PortableLONG;
typedef PortableType<int, int32_t> PortableInt;

struct PortableRECT {
    PortableRECT(const RECT &rect)
        : left(rect.left)
        , top(rect.top)
        , right(rect.right)
        , bottom(rect.bottom)
    {
    }
    operator RECT() const
    {
        RECT rect;
        rect.left = left;
        rect.top = top;
        rect.right = right;
        rect.bottom = bottom;
        return rect;
    }
    PortableLONG left;
    PortableLONG top;
    PortableLONG right;
    PortableLONG bottom;
};

struct HookEvent {
    HookEvent()
    {
        std::memset(this, 0, sizeof(HookEvent));
    }

    HookEvent(const HookEvent &o)
    {
        std::memcpy(this, &o, sizeof(HookEvent));
    }

    enum Type : uint8_t {
        CwpSize,
        CbtActivate,
        CbtCreateWindow,
        CbtDestroyWindow,
        CbtMinMax,
        CbtMoveSize,
    };

    Type type;
    union {
        struct {
            PortableHWND hwnd;
            PortableDWORD size;
        } cwpSizeData;
        struct {
            PortableHWND hwnd;
            PortableBOOL fMouse;
            PortableHWND hWndActive;
        } cbtActivateData;
        struct {
            PortableHWND hwnd;
            PortableHINSTANCE hInstance;
            PortableHMENU hMenu;
            PortableHWND hwndParent;
            PortableInt cy;
            PortableInt cx;
            PortableInt y;
            PortableInt x;
            PortableLONG style;
            PortableDWORD dwExStyle;
        } cbtCreateWindowData;
        struct {
            PortableHWND hwnd;
        } cbtDestroyWindowData;
        struct {
            PortableHWND hwnd;
            PortableInt nCmdShow;
        } cbtMinMaxData;
        struct {
            PortableHWND hwnd;
            PortableRECT rect;
        } cbtMoveSizeData;
    };
};
#pragma pack(pop)
