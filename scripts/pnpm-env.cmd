@echo off
setlocal
set "PROJECT_ROOT=%~dp0.."
set "NODE_HOME=%PROJECT_ROOT%\.toolchains\node"
set "PNPM_HOME=%PROJECT_ROOT%\.toolchains\pnpm"
set "PNPM_STORE_DIR=%PROJECT_ROOT%\.toolchains\pnpm-store"
set "CARGO_HOME=%PROJECT_ROOT%\.toolchains\cargo"
set "RUSTUP_HOME=%PROJECT_ROOT%\.toolchains\rustup"
set "NODE_EXE=%NODE_HOME%\node.exe"
set "PNPM_CLI=%PNPM_HOME%\node_modules\pnpm\bin\pnpm.cjs"

if not exist "%NODE_EXE%" (
  echo Project-isolated Node.js was not found at "%NODE_EXE%" 1>&2
  exit /b 1
)
if not exist "%PNPM_CLI%" (
  echo Project-isolated pnpm was not found at "%PNPM_CLI%" 1>&2
  exit /b 1
)

set "PATH=%PNPM_HOME%\node_modules\.bin;%NODE_HOME%;%CARGO_HOME%\bin;%PATH%"
set "PNPM_HOME=%PNPM_HOME%"
set "PNPM_STORE_DIR=%PNPM_STORE_DIR%"
set "npm_config_store_dir=%PNPM_STORE_DIR%"
for /f "delims=" %%V in ('call "%NODE_EXE%" "%PNPM_CLI%" --version') do set "PNPM_VERSION=%%V"
if /i not "%PNPM_VERSION%"=="11.15.1" (
  echo Expected project pnpm 11.15.1 but found %PNPM_VERSION% 1>&2
  exit /b 1
)

cd /d "%PROJECT_ROOT%"
"%NODE_EXE%" "%PNPM_CLI%" %*
exit /b %ERRORLEVEL%
