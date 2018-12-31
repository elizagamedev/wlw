#!/usr/bin/env python
# -*- coding: utf-8 -*-

from conans import ConanFile, CMake, tools
import os


class WtwmHookServerConan(ConanFile):
    name = "wtwm-common"
    version = "0.0.0"
    description = "Windows Tiling Window Manager - Common library"
    url = "https://github.com/elizagamedev/wtwm"
    author = "Eliza Velasquez"
    license = "GPL-3.0+"
    exports_sources = ["CMakeLists.txt", "src/*"]
    generators = "cmake"
    settings = {
        "os": ["Windows"],
        "compiler": ["Visual Studio"],
        "arch": ["x86", "x86_64"],
        "build_type": None,
    }
    requires = "boost/1.69.0@conan/stable"
    default_options = "boost:without_test=True"

    def build(self):
        cmake = CMake(self)
        cmake.configure()
        cmake.build()

    def package(self):
        self.copy("*.h", dst="include", keep_path=False)
        self.copy("*.lib", dst="lib", src="lib")

    def package_info(self):
        self.cpp_info.libs = ["wtwm_common"]
