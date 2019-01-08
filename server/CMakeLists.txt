cmake_minimum_required(VERSION 3.1)
project(wlw_server CXX)

include(${CMAKE_BINARY_DIR}/conanbuildinfo.cmake)
conan_basic_setup(TARGETS)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(wlw_server WIN32
  src/HookListener.h
  src/HookManager.cpp
  src/HookManager.h
  src/Window.cpp
  src/Window.h
  src/WindowList.cpp
  src/WindowList.h
  src/main.cpp)
target_link_libraries(wlw_server PUBLIC CONAN_PKG::wlw-common CONAN_PKG::boost)
target_include_directories(wlw_server PUBLIC src)
target_compile_definitions(wlw_server PUBLIC
  NOMINMAX)
