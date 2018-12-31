#!/usr/bin/env python
# -*- coding: utf-8 -*-

from conans import ConanFile, CMake, tools
import os


class WtwmHookServerConan(ConanFile):
    name = "wtwm-daemon"
    version = "0.0.0"
    description = "Windows Tiling Window Manager - Daemon"
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
    requires = (
        "wtwm-common/{}@eliza/testing".format(version),
        "boost/1.69.0@conan/stable",
    )
    default_options = "boost:without_test=True"

    def build(self):
        cmake = CMake(self)
        cmake.configure()
        cmake.build()

    def package(self):
        self.copy("*.exe", dst="bin", src="bin")

    def deploy(self):
        self.copy("*.exe", dst="bin", src="bin")
