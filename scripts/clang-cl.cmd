@echo off
"%~dp0..\.toolchains\llvm-mingw\bin\clang.exe" --driver-mode=cl %*
exit /b %ERRORLEVEL%
