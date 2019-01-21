cmake_minimum_required(VERSION 3.1)
project(wlw_hook C)

include(${CMAKE_BINARY_DIR}/conanbuildinfo.cmake)
conan_basic_setup(TARGETS)

if(CMAKE_SIZEOF_VOID_P EQUAL 8)
  set(bittage 64)
else()
  set(bittage 32)
endif()

add_library(wlw_hook SHARED
  dllmain.c
  HookEvent.h
  exports.def)
set_target_properties(wlw_hook PROPERTIES
  OUTPUT_NAME wlw_hook${bittage}
  C_STANDARD 99)

add_executable(size_test
  size_test.c
  HookEvent.h)
set_target_properties(size_test PROPERTIES
  C_STANDARD 99)

add_executable(worthiness_test
  worthiness_test.c)
set_target_properties(worthiness_test PROPERTIES
  C_STANDARD 99)
