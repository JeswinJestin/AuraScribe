@echo off
REM Release launcher: same native toolchain as dev.bat, but produces the installer.
REM Output: src-tauri\target\release\bundle\nsis\AuraScribe_1.0.0_x64-setup.exe
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64

REM bindgen needs libclang; whisper.cpp is compiled with CMake.
REM Quote the whole assignment — `set VAR=value && ...` captures a trailing space.
if not defined LIBCLANG_PATH set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
set "PATH=C:\Program Files\CMake\bin;%PATH%"

REM Build on most-but-not-all cores. A full whisper.cpp compile pinning every thread is the
REM heaviest sustained load this project produces, and on a thermally limited laptop it
REM throttles anyway - so the wall-clock cost of leaving headroom is small, and the machine
REM stays much cooler and quieter. Override with: set AURASCRIBE_JOBS=16
if not defined AURASCRIBE_JOBS set "AURASCRIBE_JOBS=6"
set "CARGO_BUILD_JOBS=%AURASCRIBE_JOBS%"
set "CMAKE_BUILD_PARALLEL_LEVEL=%AURASCRIBE_JOBS%"

cd /d "%~dp0"
call npm run build
