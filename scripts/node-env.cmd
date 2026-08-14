@echo off
setlocal
set "PROJECT_ROOT=%~dp0.."
set "NODE_HOME=%PROJECT_ROOT%\.toolchains\node"
set "NODE_EXE=%NODE_HOME%\node.exe"

if not exist "%NODE_EXE%" (
  echo Project-isolated Node.js was not found at "%NODE_EXE%" 1>&2
  exit /b 1
)

set "PATH=%NODE_HOME%;%PATH%"
for /f "usebackq delims=" %%V in (`"%NODE_EXE%" --version`) do set "NODE_VERSION=%%V"
if /i not "%NODE_VERSION%"=="v24.18.0" (
  echo Expected project Node.js v24.18.0 but found %NODE_VERSION% 1>&2
  exit /b 1
)

cd /d "%PROJECT_ROOT%"
"%NODE_EXE%" %*
exit /b %ERRORLEVEL%
