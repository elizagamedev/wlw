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
  exports.def)
set_target_properties(wlw_hook PROPERTIES
  OUTPUT_NAME wlw_hook${bittage})
