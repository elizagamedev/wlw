cmake_minimum_required(VERSION 3.1)
project(wtwm CXX)

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

add_library(wtwm_hook_server_dll SHARED
  dll/hooks.cpp
  dll/hooks.h
  dll/dllmain.cpp)
target_link_libraries(wtwm_hook_server_dll PUBLIC CONAN_PKG::wtwm-common CONAN_PKG::boost)
target_include_directories(wtwm_hook_server_dll PUBLIC dll)
target_compile_definitions(wtwm_hook_server_dll PUBLIC
  NOMINMAX)
target_compile_definitions(wtwm_hook_server_dll PRIVATE
  WTWM_HOOK_SERVER_DLL)
set_target_properties(wtwm_hook_server_dll PROPERTIES
  OUTPUT_NAME wtwm_hook_server${bittage})

add_executable(wtwm_hook_server_exe WIN32
  exe/main.cpp)
target_link_libraries(wtwm_hook_server_exe PUBLIC wtwm_hook_server_dll)
target_include_directories(wtwm_hook_server_exe PUBLIC exe)
target_compile_definitions(wtwm_hook_server_exe PRIVATE
  WTWM_HOOK_SERVER_EXE)
set_target_properties(wtwm_hook_server_exe PROPERTIES
  OUTPUT_NAME wtwm_hook_server${bittage})
