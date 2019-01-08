cmake_minimum_required(VERSION 3.1)
project(wlw_hook CXX)

include(${CMAKE_BINARY_DIR}/conanbuildinfo.cmake)
conan_basic_setup(TARGETS)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(CMAKE_SIZEOF_VOID_P EQUAL 8)
  set(bittage 64)
else()
  set(bittage 32)
endif()

add_library(wlw_hook_dll SHARED
  dll/hooks.cpp
  dll/hooks.h
  dll/dllmain.cpp)
target_link_libraries(wlw_hook_dll PUBLIC CONAN_PKG::wlw-common CONAN_PKG::boost)
target_include_directories(wlw_hook_dll PUBLIC dll)
target_compile_definitions(wlw_hook_dll PUBLIC
  NOMINMAX)
target_compile_definitions(wlw_hook_dll PRIVATE
  WLW_HOOK_DLL)
set_target_properties(wlw_hook_dll PROPERTIES
  OUTPUT_NAME wlw_hook${bittage})

add_executable(wlw_hook_exe WIN32
  exe/main.cpp)
target_link_libraries(wlw_hook_exe PUBLIC wlw_hook_dll)
target_include_directories(wlw_hook_exe PUBLIC exe)
target_compile_definitions(wlw_hook_exe PRIVATE
  WLW_HOOK_EXE)
set_target_properties(wlw_hook_exe PROPERTIES
  OUTPUT_NAME wlw_hook${bittage})
