# Linux wallpaper implementation

Scope: display ASIJI's rendered video/visualizer on Plasma 5/6 (X11/Wayland),
layer-shell desktops through mpvpaper, X11 desktops through xwinwrap/mpv,
and GNOME 45+ through an optional shell extension. Windows remains unchanged.

Contract: automatic selection with an explicit override; the same pause, seek,
ASCII/HD, palette and color controls as terminal playback; audio stays in ASIJI.
Backends never start a second audio stream. Missing desktop components produce
actionable errors. GNOME may require a first-install session restart.

Frames are generated in Rust and served only on loopback through a random
session URL. No media paths or control commands are exposed. Slow consumers
must not block playback or accumulate a frame queue. Shutdown stops the server
and owned processes. Plasma restores the previous wallpaper plugin without
overwriting settings changed by the user during playback. A recovery command
handles an interrupted Plasma session. Other backends overlay the wallpaper.

Code: `src/wallpaper/`, desktop assets: `linux/`, tests: Rust unit tests and
`scripts/test-linux-wallpaper.py`. Assets are embedded so portable and DEB
installations use the same code. Optional desktop tools remain distro packages.

Implementation sequence: backend selection and frame transport; Plasma;
mpvpaper/X11; GNOME; config/menu and documentation; desktop CI and packaging.
Use existing `Result`, `Command`, `TempDir` and explicit ownership; no shell
interpolation of user values, system-wide installs or driver downloads.

Validation: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`,
`cargo test --locked`, `cargo build --release --locked`, configuration/PTY tests,
and nested Linux desktop checks. A nested software-rendered desktop is not a
physical GPU or multi-monitor compatibility guarantee. Always state untested
desktop/version combinations; never bypass a failing test or rewrite user media.
