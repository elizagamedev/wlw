cmake_minimum_required(VERSION 3.1)
project(wtwm_common CXX)

include(${CMAKE_BINARY_DIR}/conanbuildinfo.cmake)
conan_basic_setup(TARGETS)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(wtwm_daemon
  # src/HookListener.h
  # src/HookManager.cpp
  # src/HookManager.h
  src/Window.cpp
  src/Window.h
  # src/WindowList.cpp
  # src/WindowList.h
  src/main.cpp)
target_link_libraries(wtwm_daemon PUBLIC CONAN_PKG::wtwm-common CONAN_PKG::boost)
target_include_directories(wtwm_daemon PUBLIC src)
target_compile_definitions(wtwm_daemon PUBLIC
  NOMINMAX)
