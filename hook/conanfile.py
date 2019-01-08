#!/usr/bin/env python
# -*- coding: utf-8 -*-

from conans import ConanFile, CMake


class WlwHookConan(ConanFile):
    name = "wlw-hook"
    version = "0.0.0"
    description = "Windows Lua Windower - Windows hook daemon"
    url = "https://github.com/elizagamedev/wlw"
    author = "Eliza Velasquez"
    license = "GPL-3.0+"
    exports_sources = ["CMakeLists.txt", "dll/*", "exe/*"]
    generators = "cmake"
    settings = {
        "os": ["Windows"],
        "compiler": ["Visual Studio"],
        "arch": ["x86", "x86_64"],
        "build_type": None,
    }
    requires = (
        "wlw-common/{}@eliza/testing".format(version),
        "boost/1.69.0@conan/stable",
    )
    default_options = "boost:without_test=True"

    def build(self):
        cmake = CMake(self)
        cmake.configure()
        cmake.build()

    def package(self):
        self.copy("*.exe", dst="bin", src="bin")
        self.copy("*.dll", dst="bin", src="bin")

    def deploy(self):
        self.copy("*.exe", dst="bin", src="bin")
        self.copy("*.dll", dst="bin", src="bin")
