@echo off
setlocal
set "PROJECT_ROOT=%~dp0.."
set "XWIN_LIB=%PROJECT_ROOT%\.toolchains\windows-msvc-sysroot-cache\windows-msvc-sysroot\windows-msvc-sysroot\lib\x86_64-unknown-windows-msvc"
rem rustc may prepend "-flavor link" for a custom LLD linker.  The project
rem toolchain ships lld-link.exe rather than the generic lld driver, so remove
rem that exact driver prefix before forwarding the remaining COFF arguments.
set "LINK_ARGS=%*"
if /i "%~1"=="-flavor" if /i "%~2"=="link" set "LINK_ARGS=%LINK_ARGS:~13%"
"%PROJECT_ROOT%\.toolchains\llvm-mingw\bin\lld-link.exe" /libpath:"%XWIN_LIB%" /defaultlib:oldnames %LINK_ARGS%
exit /b %ERRORLEVEL%
