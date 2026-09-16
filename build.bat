@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion
title Plico Build

REM ==========================================================================
REM  Plico build script
REM
REM  Usage:
REM    build.bat              release build + NSIS installer   [default]
REM    build.bat debug        debug build, much faster compile
REM    build.bat nobundle     exe only, no installer           [fastest]
REM    build.bat nopause      do not wait for keypress at the end
REM    build.bat help         show this help
REM
REM  Notes:
REM    - MSVC is located automatically and vcvars64 is loaded, so you do NOT
REM      need to open a Developer Command Prompt first.
REM    - The frontend is built automatically by tauri.conf.json's
REM      beforeBuildCommand, so you do NOT need to run npm run build first.
REM
REM  IMPORTANT: keep this file pure ASCII.
REM    cmd.exe mis-parses batch files that combine chcp 65001 with multi-byte
REM    characters (measured about 0.5 percent of runs, and much worse if a
REM    UTF-8 BOM is present). Pure ASCII never fails. The Chinese notes live
REM    in the .md documents next to this script.
REM ==========================================================================

cd /d "%~dp0"

set "PROFILE=release"
set "BUNDLE=1"
set "NOPAUSE=0"

for %%A in (%*) do (
  if /i "%%~A"=="debug"       set "PROFILE=debug"
  if /i "%%~A"=="-d"          set "PROFILE=debug"
  if /i "%%~A"=="nobundle"    set "BUNDLE=0"
  if /i "%%~A"=="--no-bundle" set "BUNDLE=0"
  if /i "%%~A"=="nopause"     set "NOPAUSE=1"
  if /i "%%~A"=="help"        goto :usage
  if /i "%%~A"=="-h"          goto :usage
  if /i "%%~A"=="--help"      goto :usage
)

set "PF86=%ProgramFiles(x86)%"
set "T=%TIME: =0%"
for /f "tokens=1-3 delims=:." %%a in ("%T%") do set /a "T0=3600*(1%%a-100)+60*(1%%b-100)+(1%%c-100)"

echo.
echo ============================================================
echo   Plico build    profile: %PROFILE%    installer: %BUNDLE%
echo ============================================================
echo.

REM --------------------------------------------------------------------------
echo [1/5] Checking environment...
REM --------------------------------------------------------------------------

if not exist "src-tauri\Cargo.toml" (
  echo    [X] Not a Plico project root - src-tauri\Cargo.toml is missing
  echo        current dir: %CD%
  goto :fail
)
echo    [OK] project root

where npm >nul 2>nul
if errorlevel 1 (
  echo    [X] npm not found
  echo        install Node.js first: https://nodejs.org/
  goto :fail
)
for /f "delims=" %%i in ('npm -v 2^>nul') do set "NPM_VER=%%i"
echo    [OK] npm !NPM_VER!

where cargo >nul 2>nul
if errorlevel 1 (
  echo    [X] cargo not found
  echo        install Rust first: https://rustup.rs/
  goto :fail
)
for /f "delims=" %%i in ('cargo -V 2^>nul') do set "CARGO_VER=%%i"
echo    [OK] !CARGO_VER!

REM ---- locate the MSVC C++ build tools ----
set "VSWHERE=%PF86%\Microsoft Visual Studio\Installer\vswhere.exe"
set "VSROOT="
if exist "%VSWHERE%" (
  for /f "delims=" %%i in ('"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath') do set "VSROOT=%%i"
)

if not defined VSROOT (
  echo    [X] MSVC C++ build tools not found
  echo.
  echo        Tauri on Windows hard-requires the MSVC toolchain.
  echo        Install it from an ADMIN PowerShell:
  echo          winget install Microsoft.VisualStudio.2022.BuildTools
  echo        Tick the workload "Desktop development with C++".
  echo.
  echo        Step by step instructions: see the dev setup .md in this folder
  goto :fail
)
echo    [OK] VS install path: !VSROOT!

set "VCVARS=!VSROOT!\VC\Auxiliary\Build\vcvars64.bat"
if not exist "!VCVARS!" (
  echo    [X] VS found but vcvars64.bat is missing
  echo        !VCVARS!
  echo        Incomplete install. Use VS Installer to add
  echo        "MSVC v143 - VS 2022 C++ x64/x86 build tools"
  goto :fail
)
echo         loading MSVC environment...
call "!VCVARS!" >nul
if errorlevel 1 (
  echo    [X] vcvars64.bat failed
  goto :fail
)

where cl.exe >nul 2>nul
if errorlevel 1 (
  echo    [X] environment loaded but cl.exe is still missing
  goto :fail
)
echo    [OK] MSVC environment ready
echo.

REM --------------------------------------------------------------------------
echo [2/5] Preparing frontend dependencies...
REM --------------------------------------------------------------------------

if exist "node_modules" (
  echo    [--] node_modules present, skipping install
) else (
  echo    installing, the first run can be slow...
  call npm install
  if errorlevel 1 (
    echo    [X] npm install failed
    goto :fail
  )
  echo    [OK] dependencies installed
)
echo.

REM --------------------------------------------------------------------------
echo [3/5] Building frontend (triggered by Tauri)
echo [4/5] Compiling Rust and bundling
REM --------------------------------------------------------------------------

set "TAURI_ARGS="
if /i "%PROFILE%"=="debug" set "TAURI_ARGS=--debug"
if "%BUNDLE%"=="0" set "TAURI_ARGS=%TAURI_ARGS% --no-bundle"

echo    run: npx tauri build %TAURI_ARGS%
echo.

REM Limit cargo parallelism: on 32GB machines with ~11GB free, parallel
REM compilation of windows/icu/brotli etc. triggers STATUS_STACK_BUFFER_OVERRUN
REM in rustc (memory pressure corrupts stack). Single job is slower but reliable.
set "CARGO_BUILD_JOBS=1"

call npx tauri build %TAURI_ARGS%
if errorlevel 1 (
  echo.
  echo    [X] Build failed. See the Tauri / cargo output above.
  goto :fail
)

REM --------------------------------------------------------------------------
echo.
echo [5/5] Artifacts
REM --------------------------------------------------------------------------

set "OUTDIR=src-tauri\target\%PROFILE%"

if exist "%OUTDIR%\Plico.exe" (
  for %%F in ("%OUTDIR%\Plico.exe") do echo    exe       : %%~fF   [%%~zF bytes]
) else if exist "%OUTDIR%\plico.exe" (
  for %%F in ("%OUTDIR%\plico.exe") do echo    exe       : %%~fF   [%%~zF bytes]
) else (
  echo    [warn] no main exe found under %OUTDIR%
)

if "%BUNDLE%"=="1" (
  set "FOUND="
  for %%F in ("%OUTDIR%\bundle\nsis\*.exe") do set "FOUND=1" & echo    installer : %%~fF   [%%~zF bytes]
  if not defined FOUND echo    [warn] no NSIS installer found
)

echo.
echo ============================================================
echo   Done
if defined T0 (
  set "T=%TIME: =0%"
  for /f "tokens=1-3 delims=:." %%a in ("!T!") do set /a "T1=3600*(1%%a-100)+60*(1%%b-100)+(1%%c-100)"
  set /a "EL=T1-T0"
  if !EL! lss 0 set /a "EL=EL+86400"
  set /a "EM=!EL!/60" & set /a "ES=!EL!%%60"
  echo   elapsed: !EM! min !ES! sec
)
echo ============================================================
echo.

if "%NOPAUSE%"=="0" (
  echo Press any key to close...
  pause >nul
)
endlocal
exit /b 0

:usage
echo.
echo   Plico build script
echo.
echo   build.bat              release build + NSIS installer [default]
echo   build.bat debug        debug build, faster compile
echo   build.bat nobundle     exe only, no installer
echo   build.bat nopause      do not wait for keypress
echo   build.bat help         show this help
echo.
echo   combinable: build.bat debug nobundle nopause
echo.
endlocal
exit /b 0

:fail
echo.
echo ============================================================
echo   Build aborted, no artifact produced
echo ============================================================
echo.
if "%NOPAUSE%"=="0" (
  echo Press any key to close...
  pause >nul
)
endlocal
exit /b 1
