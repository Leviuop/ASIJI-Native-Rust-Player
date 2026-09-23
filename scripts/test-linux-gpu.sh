#!/usr/bin/env bash
# Optional hardware test. Reports contain hardware/driver details, never media.
set -euo pipefail
PLAYER="${ASIJI_PLAYER:-asiji}"
DEVICE="${1:-/dev/dri/renderD128}"
REPORT="${2:-asiji-gpu-report.txt}"
TEMP_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TEMP_DIR"' EXIT
{
    date -u
    uname -srmo
    printf 'DRM device: %s\n' "$DEVICE"
    if command -v lspci >/dev/null; then lspci -nn | grep -E 'VGA|Display|3D' || true; fi
    if command -v vainfo >/dev/null; then vainfo --display drm --device "$DEVICE" 2>&1 || true; fi
    ffmpeg -v error -f lavfi -i 'testsrc2=s=640x360:r=30' -t 3 -c:v libx264 "$TEMP_DIR/fixture.mp4"
    "$PLAYER" --no-config --doctor --doctor-video "$TEMP_DIR/fixture.mp4" --hwaccel-device "$DEVICE"
    # A CPU fallback must not turn a failed hardware test into a pass.
    "$PLAYER" --no-config --benchmark-video "$TEMP_DIR/fixture.mp4" --hwaccel vaapi \
        --hwaccel-device "$DEVICE" --hwaccel-fallback false --mode blocks --width 160 --bench-rows 45 --bench-frames 30
} 2>&1 | tee "$REPORT"
