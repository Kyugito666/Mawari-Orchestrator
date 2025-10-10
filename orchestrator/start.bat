@echo off
title MAWARI ORCHESTRATOR

REM ## Path sudah di-set sesuai permintaan ##
set "ORCHESTRATOR_PATH=D:\SC\MyProject\Mawari-Orchestrator\orchestrator"
cd /d "%ORCHESTRATOR_PATH%"

REM ## Muat konfigurasi dari file JSON dan HAPUS SPASI ##
for /f "tokens=2 delims=:," %%a in ('findstr "main_account_username" "config\setup.json"') do set "RAW_USERNAME=%%~a"
for /f "tokens=2 delims=:," %%b in ('findstr "blueprint_repo_name" "config\setup.json"') do set "RAW_REPO_NAME=%%~b"

REM -- FIX: Trim leading spaces from variables --
for /f "tokens=* delims= " %%c in ("%RAW_USERNAME%") do set "USERNAME=%%c"
for /f "tokens=* delims= " %%d in ("%RAW_REPO_NAME%") do set "REPO_NAME=%%d"

set REPO_FULL_NAME=%USERNAME%/%REPO_NAME%

echo ==================================================
echo      CODESPACE ORCHESTRATOR
echo ==================================================
echo.
echo Akun Utama : %USERNAME%
echo Repositori : %REPO_FULL_NAME%
echo.

REM -- FIX: Add quotes around the argument to handle it as a single string --
cargo run --release -- "%REPO_FULL_NAME%"

echo.
echo Orkestrator dihentikan. Tekan tombol apa saja untuk keluar.
pause > nul

