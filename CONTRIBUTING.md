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

Linux desktop checks use isolated HOME/XDG directories, Xvfb and a separate
D-Bus session; do not run them inside your normal session bus. Install the
desktop packages listed in the workflow, then run, for example:

```sh
xvfb-run -a -s '-screen 0 1280x720x24' dbus-run-session -- python3 scripts/test-linux-wallpaper.py plasma
```

Other scenarios: `gnome`, `mpvpaper`, `x11`. The test captures actual desktop
pixels before playback, in HD/ASCII and after cleanup, using synthetic audio.
GNOME needs Shell 45+; the test starts a nested Wayland shell. Plasma 6 is
also exercised in a Debian 13 CI container. These are software-rendered
integration checks, not physical GPU/multi-monitor validation.

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
