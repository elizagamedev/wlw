#!/usr/bin/env python
# -*- coding: utf-8 -*-

from conans import ConanFile, CMake
import os


class WlwHookDllConan(ConanFile):
    name = "wlw-hook-dll"
    version = "0.1.0"
    description = "Windows Lua Windower - Windows hook dll"
    url = "https://github.com/elizagamedev/wlw"
    author = "Eliza Velasquez"
    license = "GPL-3.0+"
    exports_sources = ["CMakeLists.txt", "*.c", "*.h"]
    generators = "cmake"
    settings = {
        "os": ["Windows"],
        "compiler": None,
        "arch": ["x86", "x86_64"],
        "build_type": None,
    }

    def config_options(self):
        del self.options.fPIC

    def configure(self):
        del self.settings.compiler.libcxx

    def build(self):
        cmake = CMake(self)
        cmake.configure()
        cmake.build()

    def package(self):
        self.copy(os.path.join("*", "wlw_hook*.dll"), dst="bin", keep_path=False)

    def deploy(self):
        self.copy("*.dll", dst="bin", src="bin")
