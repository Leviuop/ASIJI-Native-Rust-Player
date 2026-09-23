param([string]$Destination = (Join-Path $PSScriptRoot '..\bin'))
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Test-Decoder([string]$Name) {
    $override = [Environment]::GetEnvironmentVariable('ASIJI_' + $Name.ToUpperInvariant())
    if ($override) {
        if (-not (Get-Command $override -ErrorAction SilentlyContinue)) {
            throw "ASIJI_$($Name.ToUpperInvariant()) does not point to an executable."
        }
        return $true
    }
    return (Test-Path -LiteralPath (Join-Path $Destination "$Name.exe")) -or
        [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

if ((Test-Decoder 'ffmpeg') -and (Test-Decoder 'ffprobe')) { exit 0 }

$version = '9.0.2'
$expectedHash = '60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba'
$url = "https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-$version-essentials_build.zip"
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$work = Join-Path $tempRoot ('asiji-ffmpeg-' + [Guid]::NewGuid().ToString('N'))
try {
    New-Item -ItemType Directory -Path $work | Out-Null
    $archive = Join-Path $work 'ffmpeg.zip'
    Write-Host "Downloading FFmpeg $version from gyan.dev (first launch only)..."
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $archive
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expectedHash) {
        throw 'FFmpeg checksum mismatch. Nothing was installed.'
    }
    Expand-Archive -LiteralPath $archive -DestinationPath $work
    $package = Join-Path $work "ffmpeg-$version-essentials_build"
    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    foreach ($name in @('ffmpeg', 'ffprobe')) {
        Copy-Item -LiteralPath (Join-Path $package "bin\$name.exe") -Destination $Destination -Force
    }
    Copy-Item -LiteralPath (Join-Path $package 'LICENSE') -Destination (Join-Path $Destination 'FFMPEG-LICENSE.txt') -Force
    Copy-Item -LiteralPath (Join-Path $package 'README.txt') -Destination (Join-Path $Destination 'FFMPEG-README.txt') -Force
    Write-Host 'FFmpeg is ready.'
} catch {
    Write-Host "FFmpeg setup failed: $_"
    Write-Host 'Install FFmpeg manually and place ffmpeg.exe and ffprobe.exe in bin, or add them to PATH.'
    exit 1
} finally {
    $resolved = [IO.Path]::GetFullPath($work)
    if ($resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path $resolved -Leaf) -match '^asiji-ffmpeg-[a-f0-9]{32}$' -and
        (Test-Path -LiteralPath $resolved)) {
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
