#!/usr/bin/env python
# -*- coding: utf-8 -*-

from conans import ConanFile, CMake


class WlwCommonConan(ConanFile):
    name = "wlw-common"
    version = "0.0.0"
    description = "Windows Lua Windower - Common library"
    url = "https://github.com/elizagamedev/wlw"
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
    requires = "Outcome/master@ned14/stable"

    def build(self):
        cmake = CMake(self)
        cmake.configure()
        cmake.build()

    def package(self):
        self.copy("*.h", dst="include", keep_path=False)
        self.copy("*.lib", dst="lib", src="lib")

    def package_info(self):
        self.cpp_info.libs = ["wlw_common"]
