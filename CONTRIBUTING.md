# Contributing

Small, focused changes are welcome. For a larger feature, open an issue first
to discuss the behavior and platform implications.

## Build and check

Use stable Rust and FFmpeg. Linux also needs the ALSA development package
(`libasound2-dev` on Debian/Ubuntu); Windows needs the MSVC C++ build tools.

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python scripts/test-licenses.py
cargo test --locked real_decode_seek_loop_and_cache -- --ignored
cargo test --locked unavailable_gpu_falls_back_unless_strict -- --ignored
cargo build --release --locked
python scripts/test-config.py
python scripts/test-import.py
python scripts/test-settings.py
python scripts/test-settings-terminal.py
python scripts/test-doctor.py
```

Playback checks need an audio output:

```sh
cargo test --locked real_audio_pause_seek_end_and_restart -- --ignored
python scripts/test-playback.py
```

On Windows the terminal test needs `pywinpty`. For headless Linux, set
`ALSA_CONFIG_PATH` to the absolute path of `tests/alsa-null.conf`.
The null output verifies playback logic, not sound quality or physical devices.
See [hardware validation](docs/HARDWARE.md) for GPU checks.

Wallpaper checks require an interactive Windows Explorer desktop (not headless CI):

```sh
cargo test --locked desktop_buffer_renders_and_window_is_removed -- --ignored
python scripts/test-wallpaper.py
```

Close other ASIJI wallpaper sessions first. The tests briefly create their own
desktop window and use synthetic audio at zero volume.

## Pull requests

Describe the problem, resulting behavior and checks you ran. Include regression
coverage where useful. Keep unrelated formatting or refactoring separate.
Changes to options or shortcuts should update the config example and user guide.

Do not commit personal media, audio caches, binaries, local configuration or
credentials. Use short synthetic FFmpeg fixtures in tests. Keep third-party
notices intact. Code contributions are distributed under the project's MIT license.

Submit only material you are entitled to contribute under these terms.
Identify third-party code and retain its original notices; do not relabel it
as project-authored code. Review [licensing guidance](docs/LICENSING.md) before
changing dependencies or release contents. Release packaging requires
`rustup component add rust-docs` for standard-library notices.
