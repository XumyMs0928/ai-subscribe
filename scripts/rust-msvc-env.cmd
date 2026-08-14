@echo off
setlocal
set "PROJECT_ROOT=%~dp0.."
set "CARGO_HOME=%PROJECT_ROOT%\.toolchains\cargo"
set "RUSTUP_HOME=%PROJECT_ROOT%\.toolchains\rustup"
set "CARGO=%CARGO_HOME%\bin\cargo.exe"
set "XWIN_CACHE_DIR=%PROJECT_ROOT%\.toolchains\windows-msvc-sysroot-cache"
set "XWIN_SYSROOT=%XWIN_CACHE_DIR%\windows-msvc-sysroot\windows-msvc-sysroot"
set "PATH=%CARGO_HOME%\bin;%PROJECT_ROOT%\.toolchains\llvm-mingw\bin;%PATH%"
set "LIB=%XWIN_SYSROOT%\lib\x86_64-unknown-windows-msvc"
set "INCLUDE=%XWIN_SYSROOT%\include"
set "RC_x86_64_pc_windows_msvc=%PROJECT_ROOT%\.toolchains\llvm-mingw\bin\llvm-rc.exe"
set "RC=%PROJECT_ROOT%\.toolchains\llvm-mingw\bin\llvm-rc.exe"
set "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=%PROJECT_ROOT%\.toolchains\llvm-mingw\bin\lld-link.exe"
set "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS=-C linker-flavor=lld-link -C link-arg=-defaultlib:oldnames"
set "CARGO_BUILD_TARGET=x86_64-pc-windows-msvc"
set "CC_x86_64_pc_windows_msvc=%PROJECT_ROOT%\scripts\clang-cl.cmd"
set "CXX_x86_64_pc_windows_msvc=%PROJECT_ROOT%\scripts\clang-cl.cmd"
set "CC=%PROJECT_ROOT%\scripts\clang-cl.cmd"
set "CXX=%PROJECT_ROOT%\scripts\clang-cl.cmd"

if not exist "%CARGO%" (
  echo Project-isolated Cargo was not found at "%CARGO%" 1>&2
  exit /b 1
)

cd /d "%PROJECT_ROOT%"
"%CARGO%" %*
exit /b %ERRORLEVEL%
