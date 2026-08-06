@echo off
REM Dev launcher WITH the Moonshine engine (ONNX via sherpa-onnx) compiled in.
REM Same native toolchain as dev.bat, plus --features moonshine. Use this to try the new
REM engine and confirm a real transcript before deciding whether it ships by default.
REM The first build downloads a prebuilt sherpa-onnx + onnxruntime (cached afterwards).
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64

if not defined LIBCLANG_PATH set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
set "PATH=C:\Program Files\CMake\bin;%PATH%"

if not defined AURASCRIBE_JOBS set "AURASCRIBE_JOBS=6"
set "CARGO_BUILD_JOBS=%AURASCRIBE_JOBS%"
set "CMAKE_BUILD_PARALLEL_LEVEL=%AURASCRIBE_JOBS%"

cd /d "%~dp0"
npx tauri dev --features moonshine
