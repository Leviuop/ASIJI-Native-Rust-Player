param([string]$Destination = (Join-Path $env:LOCALAPPDATA 'Programs\ASIJI'), [switch]$NoShortcut)
$ErrorActionPreference = 'Stop'
$source = Split-Path $PSScriptRoot -Parent
if (!(Test-Path -LiteralPath (Join-Path $source 'bin\asiji.exe'))) {
    throw 'Extract a Windows release first, or run build.bat.'
}
$destinationPath = [IO.Path]::GetFullPath($Destination)
if ($destinationPath.TrimEnd('\') -eq ([IO.Path]::GetFullPath($source)).TrimEnd('\')) {
    throw 'Choose an installation directory different from the extracted archive.'
}
New-Item -ItemType Directory -Force -Path $destinationPath | Out-Null
foreach ($folder in @('bin', 'scripts', 'media')) {
    New-Item -ItemType Directory -Force -Path (Join-Path $destinationPath $folder) | Out-Null
}
# Copy only distributable files: never a user's media, cache, or config.
foreach ($file in @('README.md', 'LICENSE', 'THIRD_PARTY.md', 'CHANGELOG.md', 'config.example.toml', 'start.bat', 'install.bat')) {
    Copy-Item -LiteralPath (Join-Path $source $file) -Destination (Join-Path $destinationPath $file) -Force
}
Copy-Item -LiteralPath (Join-Path $source 'bin\asiji.exe') -Destination (Join-Path $destinationPath 'bin\asiji.exe') -Force
Copy-Item -LiteralPath (Join-Path $source 'media\README.txt') -Destination (Join-Path $destinationPath 'media\README.txt') -Force
foreach ($file in @('setup-ffmpeg.ps1', 'install-windows.ps1')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $file) -Destination (Join-Path $destinationPath "scripts\$file") -Force
}
foreach ($folder in @('licenses', 'sources')) {
    if (Test-Path -LiteralPath (Join-Path $source $folder)) {
        Copy-Item -LiteralPath (Join-Path $source $folder) -Destination $destinationPath -Recurse -Force
    }
}
if (!$NoShortcut) {
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Programs')) 'ASIJI.lnk'))
    $shortcut.TargetPath = Join-Path $destinationPath 'start.bat'
    $shortcut.WorkingDirectory = $destinationPath
    $shortcut.Save()
}
Write-Host "Installed: $destinationPath"
Write-Host 'Launch start.bat or the Start menu shortcut. Missing FFmpeg is downloaded on first launch.'
Write-Host 'Your existing config and media are preserved. No drivers or system settings were changed.'
