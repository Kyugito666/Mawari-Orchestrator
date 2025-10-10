@echo off
title Mawari Orchestrator Monitor
color 0A

REM ## Ganti PATH di bawah ini dengan path ke folder 'orchestrator' Anda ##
set "ORCHESTRATOR_PATH=C:\path\to\your\Mawari-Orchestrator\orchestrator"

:loop
cls
echo =================================================
echo             MAWARI MONITOR
echo =================================================
echo Waktu: %date% %time%
echo.

cd /d "%ORCHESTRATOR_PATH%"

echo -------------------------------------------------
echo STATUS ORKESTRATOR (state.json):
echo -------------------------------------------------
if exist state.json (
    type state.json
) else (
    echo Tidak ada file state.json ditemukan.
)

echo.
echo -------------------------------------------------
echo CODESPACES MAWARI AKTIF (via gh cs list):
echo -------------------------------------------------
gh cs list | findstr "mawari"

echo.
echo -------------------------------------------------
echo Refresh dalam 60 detik... (Tekan Ctrl+C untuk berhenti)
timeout /t 60 /nobreak >nul
goto loop
