# Third-party components

The project's MIT license covers ASIJI code and documentation; it does not
replace third-party terms. ASIJI is not made solely from its maintainer's code.
See [docs/LICENSING.md](docs/LICENSING.md) for the audit scope and redistribution guidance.

ASIJI uses Rust libraries listed in `Cargo.lock`, including rodio, crossterm,
clap, hound and RustFFT. Each binary distribution includes platform-specific
`licenses/INDEX.txt` and `licenses/DEPENDENCIES.json` with versions, upstream
repositories, license expressions and notice paths. Build dependencies are
included; this is not a binary symbol inventory. Unsupported-target
dependencies are excluded from the release inventory.

MIT is selected where offered as an alternative; standalone Apache-2.0 and
MPL-2.0 components retain those terms. Conjunctive requirements remain:
unicode-ident also requires Unicode-3.0; encoding_rs includes BSD-3-Clause
material. Original notices include LICENSE-UNICODE and LICENSE-WHATWG.
`licenses/dasp_sample-LICENSE-MIT.txt` supplies the upstream RustAudio notice
omitted from the dasp_sample crate archive; it is taken from the crate's
original source commit `97c3bb9b2363c0b46ac1633858bf1054fd02a980`.
The upstream notice is available [at that commit](https://raw.githubusercontent.com/RustAudio/dasp/97c3bb9b2363c0b46ac1633858bf1054fd02a980/LICENSE-MIT).

## Symphonia (MPL-2.0)

The audio decoder includes unmodified Symphonia 0.5.5 components, licensed
under the Mozilla Public License 2.0. Their source code remains available
under MPL-2.0; ASIJI's MIT license does not replace those terms.

The complete original source archives are included in `sources/` inside each
binary distribution (under `/usr/share/doc/asiji/` for the Debian package).
Each `.crate` is a standard gzip-compressed tar archive. Original sources:

- [symphonia 0.5.5](https://crates.io/api/v1/crates/symphonia/0.5.5/download)
- [symphonia-core 0.5.5](https://crates.io/api/v1/crates/symphonia-core/0.5.5/download)
- [symphonia-codec-pcm 0.5.5](https://crates.io/api/v1/crates/symphonia-codec-pcm/0.5.5/download)
- [symphonia-format-riff 0.5.5](https://crates.io/api/v1/crates/symphonia-format-riff/0.5.5/download)
- [symphonia-metadata 0.5.5](https://crates.io/api/v1/crates/symphonia-metadata/0.5.5/download)

Other dependency sources can be obtained with `cargo fetch --locked` using
this project's `Cargo.lock`, or from their versioned crates.io downloads.
Keep all third-party license, copyright and notice files when redistributing.

## Rust standard library and system runtime

`licenses/rust-standard-library/` contains the standard-library copyright
report, license texts and compiler/target details from the build's Rust toolchain.
These components retain their notices and exceptions.

Windows MSVC builds statically link the Microsoft CRT under the applicable
toolchain terms; this runtime is not MIT-licensed. Linux relies on dynamically
linked system glibc and ALSA; those libraries are not bundled. See the licensing
guide for upstream terms.

## Optional Linux desktop components

The Plasma QML plugin and GNOME extension under `linux/` are ASIJI source,
distributed with the project's MIT notice. Their per-user installation includes
that notice. The small ASCII pixel masks are project-authored; no font file is
copied into the package.

Plasma/Qt, GNOME Shell, [mpv](https://mpv.io/),
[mpvpaper](https://github.com/GhostNaN/mpvpaper) and
[xwinwrap](https://github.com/mmhobi7/xwinwrap) are optional system components.
Their source or binaries are not bundled in ASIJI archives. They retain their
own licenses; ASIJI's MIT license does not cover them. In particular, mpvpaper
is GPL-3.0. Obtain them through the distribution or their upstream installation
instructions and preserve their terms when redistributing a combined bundle.

## FFmpeg

FFmpeg and FFprobe run as separate programs. They are not included in ASIJI
release archives. The Windows launcher downloads the FFmpeg 9.0.2 essentials
build directly from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) if neither
local binaries nor a system installation is available. The archive is checked
against a pinned SHA-256 checksum. Its GPLv3 license and build information are
installed alongside the executables. FFmpeg sources and build details are
linked in that distribution's README.

Do not treat an installed folder containing downloaded FFmpeg as an MIT-only
redistribution. Redistributing FFmpeg needs a separate review of the exact
build and its corresponding-source obligations. A generic main-branch URL
is not sufficient evidence of compliance.

On Linux, install FFmpeg through your distribution's package manager.
Media files are supplied by the user and are not part of this project.

## Demonstration artwork

The PNG/GIF examples use native renderer frames and synthetic audio, not user
recordings. Only rasterized text is included, not font files.
See [artwork provenance](docs/assets/README.md). Compatibility references
do not imply endorsement by trademark owners.
