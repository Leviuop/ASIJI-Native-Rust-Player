#!/usr/bin/env bash
set -euo pipefail
APP_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

if [[ ! -t 0 || ! -t 1 ]]; then
    if command -v x-terminal-emulator >/dev/null 2>&1; then
        exec x-terminal-emulator -e bash "$APP_DIR/start.sh" "$@"
    elif command -v gnome-terminal >/dev/null 2>&1; then
        exec gnome-terminal -- bash "$APP_DIR/start.sh" "$@"
    elif command -v konsole >/dev/null 2>&1; then
        exec konsole -e bash "$APP_DIR/start.sh" "$@"
    elif command -v xterm >/dev/null 2>&1; then
        exec xterm -e bash "$APP_DIR/start.sh" "$@"
    fi
    echo 'Open a terminal and run: bash start.sh' >&2
    exit 1
fi

cd -- "$APP_DIR"
if [[ ! -x bin/asiji ]]; then
    if ! bash build.sh; then
        read -r -p 'Press Enter to close...' _
        exit 1
    fi
fi
if ./bin/asiji "$@"; then
    exit 0
else
    result=$?
    echo 'On Debian/Ubuntu, check: sudo apt install ffmpeg libasound2'
    read -r -p 'Press Enter to close...' _
    exit "$result"
fi
