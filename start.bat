@echo off
setlocal
chcp 65001 >nul
title ASIJI - Native Rust Player
cd /d "%~dp0"
if exist "bin\asiji.exe" goto run
call build.bat
if errorlevel 1 goto finished
:run
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\setup-ffmpeg.ps1"
if errorlevel 1 goto finished
"bin\asiji.exe" %*
:finished
set "result=%errorlevel%"
if not "%result%"=="0" pause
exit /b %result%
