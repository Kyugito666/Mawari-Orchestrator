@echo off
title MAWARI ORCHESTRATOR

REM ## Ganti PATH di bawah ini dengan path ke folder 'orchestrator' Anda ##
set "ORCHESTRATOR_PATH=C:\path\to\your\Mawari-Orchestrator\orchestrator"
cd /d "%ORCHESTRATOR_PATH%"

REM ## Muat konfigurasi dari file JSON (PATH DIPERBAIKI) ##
for /f "tokens=2 delims=:," %%a in ('findstr "main_account_username" "config\config_setup.json"') do set "USERNAME=%%~a"
for /f "tokens=2 delims=:," %%b in ('findstr "blueprint_repo_name" "config\config_setup.json"') do set "REPO_NAME=%%~b"

set USERNAME=%USERNAME:"=%
set REPO_NAME=%REPO_NAME:"=%
set REPO_FULL_NAME=%USERNAME%/%REPO_NAME%

echo ==================================================
echo      MAWARI MULTI-CODESPACE ORCHESTRATOR
echo ==================================================
echo.
echo Akun Utama : %USERNAME%
echo Repositori : %REPO_FULL_NAME%
echo.

cargo run --release -- %REPO_FULL_NAME%

echo.
echo Orkestrator dihentikan. Tekan tombol apa saja untuk keluar.
pause > nul
