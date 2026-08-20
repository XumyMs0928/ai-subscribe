@echo off
setlocal
set "PROJECT_ROOT=%~dp0.."
set "CARGO_HOME=%PROJECT_ROOT%\.toolchains\cargo"
set "RUSTUP_HOME=%PROJECT_ROOT%\.toolchains\rustup"
set "XWIN_SYSROOT=%PROJECT_ROOT%\.toolchains\windows-msvc-sysroot-cache\windows-msvc-sysroot\windows-msvc-sysroot"
set "PATH=%CARGO_HOME%\bin;%PROJECT_ROOT%\.toolchains\llvm-mingw\bin;%PATH%"
set "CARGO=%CARGO_HOME%\bin\cargo.exe"
set "GNU_COMPAT_LIB=%PROJECT_ROOT%\.toolchains\llvm-mingw-compat"
if not exist "%GNU_COMPAT_LIB%" mkdir "%GNU_COMPAT_LIB%"
if not exist "%GNU_COMPAT_LIB%\liboldnames.a" copy /y "%PROJECT_ROOT%\.toolchains\llvm-mingw\x86_64-w64-mingw32\lib\libmoldname.a" "%GNU_COMPAT_LIB%\liboldnames.a" >nul
set "INCLUDE=%XWIN_SYSROOT%\include"
set "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=%PROJECT_ROOT%\scripts\lld-link-xwin.cmd"
set "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS=-C linker-flavor=lld-link"
set "CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_RUSTFLAGS=-Lnative=%GNU_COMPAT_LIB%"
set "RC_x86_64_pc_windows_gnullvm=%PROJECT_ROOT%\.toolchains\llvm-mingw\bin\llvm-rc.exe"
set "RC=%PROJECT_ROOT%\.toolchains\llvm-mingw\bin\llvm-rc.exe"
set "CC_x86_64_pc_windows_msvc=%PROJECT_ROOT%\scripts\clang-cl.cmd"
set "CXX_x86_64_pc_windows_msvc=%PROJECT_ROOT%\scripts\clang-cl.cmd"
set "CC=%PROJECT_ROOT%\scripts\clang-cl.cmd"
set "CXX=%PROJECT_ROOT%\scripts\clang-cl.cmd"
cd /d "%PROJECT_ROOT%"

if not exist "%CARGO%" (
  echo Project-isolated Cargo was not found at "%CARGO%" 1>&2
  exit /b 1
)

"%CARGO%" %*
exit /b %ERRORLEVEL%
