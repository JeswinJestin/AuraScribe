@echo off
REM Release launcher: same native toolchain as dev.bat, but produces the installer.
REM Output: src-tauri\target\release\bundle\nsis\AuraScribe_1.0.0_x64-setup.exe
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64

REM bindgen needs libclang; whisper.cpp is compiled with CMake.
REM Quote the whole assignment — `set VAR=value && ...` captures a trailing space.
if not defined LIBCLANG_PATH set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
set "PATH=C:\Program Files\CMake\bin;%PATH%"

cd /d "%~dp0"
call npm run build
