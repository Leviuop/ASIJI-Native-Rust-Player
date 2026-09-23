# Third-party components

ASIJI uses Rust libraries listed in `Cargo.lock`, including rodio, crossterm,
clap and their dependencies. Binary archives include their available license
and notice files under `licenses/`.
`licenses/dasp_sample-LICENSE-MIT.txt` supplies the upstream RustAudio notice
omitted from the dasp_sample crate archive; it is taken from the crate's
original source commit `97c3bb9b2363c0b46ac1633858bf1054fd02a980`.

## Symphonia (MPL-2.0)

The audio decoder includes unmodified Symphonia 0.5.5 components, licensed
under the Mozilla Public License 2.0. Their source code remains available
under MPL-2.0; ASIJI's MIT license does not replace those terms.

The complete original source archives are provided in the release asset
`asiji-0.3.1-third-party-sources.zip`, alongside the binary downloads:
[download sources](https://github.com/Leviuop/ASIJI-Native-Rust-Player/releases/download/v0.3.1/asiji-0.3.1-third-party-sources.zip).
Each `.crate` is a standard gzip-compressed tar archive. Original sources:

- [symphonia 0.5.5](https://crates.io/api/v1/crates/symphonia/0.5.5/download)
- [symphonia-core 0.5.5](https://crates.io/api/v1/crates/symphonia-core/0.5.5/download)
- [symphonia-codec-pcm 0.5.5](https://crates.io/api/v1/crates/symphonia-codec-pcm/0.5.5/download)
- [symphonia-format-riff 0.5.5](https://crates.io/api/v1/crates/symphonia-format-riff/0.5.5/download)
- [symphonia-metadata 0.5.5](https://crates.io/api/v1/crates/symphonia-metadata/0.5.5/download)

Other dependency sources can be obtained with `cargo fetch --locked` using
this project's `Cargo.lock`, or from their versioned crates.io downloads.
Keep all third-party license, copyright and notice files when redistributing.

## FFmpeg

FFmpeg and FFprobe run as separate programs. They are not included in ASIJI
release archives. The Windows launcher downloads the FFmpeg 9.0.2 essentials
build directly from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) if neither
local binaries nor a system installation is available. The archive is checked
against a pinned SHA-256 checksum. Its GPLv3 license and build information are
installed alongside the executables. FFmpeg sources and build details are
linked in that distribution's README.

On Linux, install FFmpeg through your distribution's package manager.
Media files are supplied by the user and are not part of this project.
