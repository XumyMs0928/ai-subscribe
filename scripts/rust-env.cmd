@echo off
setlocal
set "PROJECT_ROOT=%~dp0.."
set "CARGO_HOME=%PROJECT_ROOT%\.toolchains\cargo"
set "RUSTUP_HOME=%PROJECT_ROOT%\.toolchains\rustup"
set "PATH=%PROJECT_ROOT%\.toolchains\llvm-mingw\bin;%PATH%"
set "CARGO=%CARGO_HOME%\bin\cargo.exe"
cd /d "%PROJECT_ROOT%"

if not exist "%CARGO%" (
  echo Project-isolated Cargo was not found at "%CARGO%" 1>&2
  exit /b 1
)

"%CARGO%" %*
exit /b %ERRORLEVEL%
