#!/usr/bin/env bash
set -euo pipefail
APP_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEST="${1:-${XDG_DATA_HOME:-$HOME/.local/share}/asiji}"
BIN_DIR="${ASIJI_BIN_DIR:-$HOME/.local/bin}"
[[ -x "$APP_DIR/bin/asiji" ]] || { echo 'Extract a Linux release or run bash build.sh first.' >&2; exit 1; }
mkdir -p -- "$DEST" "$BIN_DIR"
DEST="$(cd -- "$DEST" && pwd)"
[[ "$DEST" != "$APP_DIR" ]] || { echo 'Choose a directory different from the extracted archive.' >&2; exit 1; }
mkdir -p -- "$DEST/bin" "$DEST/media"
# Keep personal media, caches and settings out of the installation copy.
install -m 755 -- "$APP_DIR/bin/asiji" "$DEST/bin/asiji"
install -m 755 -- "$APP_DIR/start.sh" "$APP_DIR/install.sh" "$DEST/"
for file in README.md LICENSE THIRD_PARTY.md CHANGELOG.md config.example.toml; do
    install -m 644 -- "$APP_DIR/$file" "$DEST/$file"
done
install -m 644 -- "$APP_DIR/media/README.txt" "$DEST/media/README.txt"
for folder in licenses sources docs; do
    [[ ! -d "$APP_DIR/$folder" ]] || cp -R -- "$APP_DIR/$folder" "$DEST/"
done
if [[ -e "$BIN_DIR/asiji" || -L "$BIN_DIR/asiji" ]]; then
    [[ -L "$BIN_DIR/asiji" && "$(readlink -- "$BIN_DIR/asiji")" == "$DEST/bin/asiji" ]] || {
        echo "Refusing to replace existing $BIN_DIR/asiji; installed files are in $DEST" >&2; exit 1;
    }
else
    ln -s -- "$DEST/bin/asiji" "$BIN_DIR/asiji"
fi
printf 'Installed: %s\nRun: %s/asiji\nAdd %s to PATH if needed.\n' "$DEST" "$BIN_DIR" "$BIN_DIR"
printf 'Install FFmpeg, ALSA and optional VAAPI drivers with your distribution package manager.\n'

if [[ -f "$APP_DIR/scripts/test-linux-gpu.sh" ]]; then
    mkdir -p -- "$DEST/scripts"
    install -m 644 -- "$APP_DIR/scripts/test-linux-gpu.sh" "$DEST/scripts/test-linux-gpu.sh"
fi
