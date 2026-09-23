@echo off
setlocal
cd /d "%~dp0"
if exist ".tools\cargo\bin\cargo.exe" (
    set "CARGO_HOME=%CD%\.tools\cargo"
    set "RUSTUP_HOME=%CD%\.tools\rustup"
    set "PATH=%CD%\.tools\cargo\bin;%PATH%"
)
where cargo >nul 2>nul
if errorlevel 1 (
    echo Install Rust and Visual Studio C++ Build Tools: https://rustup.rs/
    exit /b 1
)
cargo build --release --locked
if errorlevel 1 exit /b 1
if not exist bin mkdir bin
copy /y "target\release\asiji.exe" "bin\asiji.exe" >nul
if errorlevel 1 exit /b 1
echo Built bin\asiji.exe
