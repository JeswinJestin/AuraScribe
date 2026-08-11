@echo off
REM Release launcher: produces the SHIPPING installer with the Moonshine engine (and Parakeet,
REM Dolphin, NeMo-CTC) compiled in and the sherpa-onnx + ONNX Runtime + MSVC runtime DLLs bundled.
REM v1.0.0 ships Moonshine by default, so this is the ONLY correct release build — a plain
REM `tauri build` on the base config omits onnxruntime.dll and the app fails on launch.
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
REM Build the frontend, then the installer WITH the Moonshine overlay config. --config merges the
REM overlay that bundles the sherpa-onnx + ONNX Runtime DLLs next to the exe (see
REM tauri.moonshine.conf.json), without which the installed app cannot load its models.
call npm run build:frontend
call npx tauri build --features moonshine --config src-tauri/tauri.moonshine.conf.json
