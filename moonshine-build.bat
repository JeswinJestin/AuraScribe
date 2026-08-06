@echo off
REM Release build WITH the Moonshine engine compiled in. Produces an installer that bundles
REM the ONNX Runtime, so it is meaningfully larger than the Whisper-only build.bat output
REM (the ~4.6 MB figure is the Whisper-only installer). Only ship this if the larger size is
REM an accepted trade for Moonshine's speed/multilingual gains.
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64

if not defined LIBCLANG_PATH set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
set "PATH=C:\Program Files\CMake\bin;%PATH%"

if not defined AURASCRIBE_JOBS set "AURASCRIBE_JOBS=6"
set "CARGO_BUILD_JOBS=%AURASCRIBE_JOBS%"
set "CMAKE_BUILD_PARALLEL_LEVEL=%AURASCRIBE_JOBS%"

cd /d "%~dp0"
REM --config merges the Moonshine overlay, which bundles the sherpa-onnx + ONNX Runtime DLLs
REM next to the exe so the installed app can find them at runtime.
npx tauri build --features moonshine --config src-tauri/tauri.moonshine.conf.json
