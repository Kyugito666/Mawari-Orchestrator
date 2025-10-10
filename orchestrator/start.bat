@echo off
title MAWARI ORCHESTRATOR

REM ## Ganti PATH di bawah ini dengan path ke folder 'orchestrator' Anda ##
set "ORCHESTRATOR_PATH=D:\SC\MyProject\Mawari-Orchestrator\orchestrator"
cd /d "%ORCHESTRATOR_PATH%"

REM Parsing argumen dengan aman untuk menghindari spasi
set "REPO_FULL_NAME=%~1"

echo ==================================================
echo      MAWARI 12-NODE ORCHESTRATOR
echo ==================================================
echo.
echo Repositori Target: "%REPO_FULL_NAME%"
echo.

REM Jalankan orchestrator dengan argumen yang sudah di-quote
cargo run --release -- "%REPO_FULL_NAME%"

echo.
echo Orkestrator dihentikan. Tekan tombol apa saja untuk keluar.
pause > nul

