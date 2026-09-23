# Third-party components

ASIJI uses Rust libraries listed in `Cargo.lock`, including rodio, crossterm,
clap and their dependencies. Binary archives include their available license
and notice files under `licenses/`.

FFmpeg and FFprobe run as separate programs. They are not included in ASIJI
release archives. The Windows launcher downloads the FFmpeg 9.0.2 essentials
build directly from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) if neither
local binaries nor a system installation is available. The archive is checked
against a pinned SHA-256 checksum. Its GPLv3 license and build information are
installed alongside the executables. FFmpeg sources and build details are
linked in that distribution's README.

On Linux, install FFmpeg through your distribution's package manager.
Media files are supplied by the user and are not part of this project.
