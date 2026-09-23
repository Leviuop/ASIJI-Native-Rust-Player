# Changelog

## 0.4.0

- Add a commented TOML configuration with XDG paths on Linux and AppData on Windows.
- Add configurable playback shortcuts, conflict checks and matching on-screen hints.
- Add `--init-config`, `--print-config`, `--config` and `--no-config`.
- Allow CLI overrides for volume, mute, repeat and color; retain 10% default volume.
- Include the config example, third-party notices and MPL dependency sources in release archives.

## 0.3.1

- Clear the menu and scrollback on refresh and return from playback.
- Keep playback errors visible in the refreshed menu.
- Add Windows and Linux release archives and CI checks.
- Install missing Windows FFmpeg binaries on first launch with SHA-256 verification.
- Support the FFprobe version shipped with Ubuntu 22.04.

## 0.3.0

- Select CPU, CUDA or D3D11VA by measuring decoding speed for each clip.
- Render only changed cells and reuse ANSI colors and buffers.
- Schedule frames against the audio clock and improve Windows timer precision.
- Limit frame rate to the source and set initial volume to 10%.

## 0.2.0

- Native Rust player with synchronized audio, ASCII and half-block video,
  seeking, volume, pause, repeat and automatic media pairing.
