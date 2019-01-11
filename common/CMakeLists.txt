cmake_minimum_required(VERSION 3.1)
project(wlw_common CXX)

include(${CMAKE_BINARY_DIR}/conanbuildinfo.cmake)
conan_basic_setup(TARGETS)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(wlw_common STATIC
  src/HookEvent.h
  src/MainWindow.cpp
  src/MainWindow.h
  src/WindowsError.cpp
  src/WindowsError.h
  src/WindowsHook.cpp
  src/WindowsHook.h
  src/win32.cpp
  src/win32.h)
target_link_libraries(wlw_common PUBLIC CONAN_PKG::Outcome)
target_include_directories(wlw_common PUBLIC src)
target_compile_definitions(wlw_common PUBLIC
  NOMINMAX)
set_target_properties(wlw_common PROPERTIES
  WINDOWS_EXPORT_ALL_SYMBOLS ON)
