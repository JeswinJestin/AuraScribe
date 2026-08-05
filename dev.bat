@echo off
REM Dev launcher: sets up the native toolchain whisper-rs needs, then starts Tauri.
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64

REM bindgen needs libclang; whisper.cpp is compiled with CMake.
if not defined LIBCLANG_PATH set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
set "PATH=C:\Program Files\CMake\bin;%PATH%"

cd /d "%~dp0"
npx tauri dev
