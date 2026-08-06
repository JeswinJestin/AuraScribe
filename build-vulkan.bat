@echo off
REM GPU build: same toolchain as build.bat, but compiles whisper.cpp with the Vulkan backend
REM so transcription runs on the GPU instead of the CPU. Requires the Vulkan SDK installed
REM (VULKAN_SDK set); without it the whisper-rs-sys build fails looking for Vulkan headers.
REM
REM The resulting binary is portable across NVIDIA/AMD/Intel GPUs. On a machine with no
REM Vulkan-capable GPU it falls back to CPU at runtime, so it is safe to ship as the default.
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64

if not defined LIBCLANG_PATH set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"

REM Build whisper.cpp's Vulkan backend with the CMake 3.31 that ships with VS Build Tools,
REM NOT the system CMake 4.4. The Vulkan build compiles a host tool (vulkan-shaders-gen) via
REM a nested ExternalProject whose compiler-ABI probe fails under CMake 4.x — even with the
REM `CMAKE_POLICY_VERSION_MINIMUM=3.5` pin that makes the top-level build configure. CMake
REM 3.31 is what whisper.cpp's build was written against and configures the nested project
REM cleanly. Prepended so it wins over C:\Program Files\CMake\bin.
set "VSCMAKE=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin"
set "VSNINJA=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja"
set "PATH=%VSCMAKE%;%VSNINJA%;%PATH%"

REM Ninja, not MSBuild: MSBuild fails vulkan-shaders-gen's custom build step with
REM "MSB4184 / items cannot be built in parallel". Ninja handles the cross-target dependency.
set "CMAKE_GENERATOR=Ninja"

REM Cap parallelism so a thermally limited laptop stays cool during the whisper.cpp compile.
if not defined AURASCRIBE_JOBS set "AURASCRIBE_JOBS=6"
set "CARGO_BUILD_JOBS=%AURASCRIBE_JOBS%"
set "CMAKE_BUILD_PARALLEL_LEVEL=%AURASCRIBE_JOBS%"

cd /d "%~dp0"
if not defined VULKAN_SDK (
  echo ERROR: VULKAN_SDK is not set. Install the Vulkan SDK from vulkan.lunarg.com first.
  exit /b 1
)
echo Building with Vulkan GPU backend using VULKAN_SDK=%VULKAN_SDK%
call npx tauri build --features vulkan
