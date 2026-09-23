"""Check configuration CLI behavior without FFmpeg or an audio device."""

import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
binary = root / "target/release" / ("asiji.exe" if os.name == "nt" else "asiji")
with tempfile.TemporaryDirectory() as temporary:
    folder = Path(temporary)
    environment = dict(os.environ, APPDATA=str(folder), XDG_CONFIG_HOME=str(folder),
                       HOME=str(folder), ASIJI_FFMPEG="missing-ffmpeg-for-config-test")

    def run(*arguments, success=True):
        result = subprocess.run([str(binary), *arguments], env=environment,
                                capture_output=True, encoding="utf-8", timeout=10)
        assert (result.returncode == 0) == success, result.stdout + result.stderr
        return result.stdout + result.stderr

    default = folder / "asiji/config.toml"
    assert "volume = 10" in run("--print-config")
    run("--init-config")
    original = default.read_bytes()
    run("--init-config", success=False)
    assert default.read_bytes() == original
    default.write_text('volume=37\nfps=60\ncolor=false\nmedia="music"\n'
                       '[bindings]\npause=["k"]\n', encoding="utf-8")
    effective = run("--print-config")
    assert "volume = 37" in effective and "fps = 60" in effective
    assert 'pause = ["k"]' in effective
    assert "volume = 10" in run("--volume", "10", "--print-config")
    assert "color = true" in run("--color", "--print-config")
    assert "volume = 10" in run("--no-config", "--print-config")
    default.write_text('visualizer="orbit"\nvisual_theme="ember"\nvisual_gain=180\nvisual_smoothing=90\nvisual_bands=64\n', encoding="utf-8")
    effective = run("--print-config")
    assert 'visualizer = "orbit"' in effective and 'visual_gain = 180' in effective
    overrides = run("--visualizer", "bars", "--visual-theme", "ice", "--visual-gain", "100", "--visual-smoothing", "80", "--visual-bands", "48", "--print-config")
    for field in ('visualizer = "bars"', 'visual_theme = "ice"', 'visual_gain = 100', 'visual_smoothing = 80', 'visual_bands = 48'):
        assert field in overrides, overrides
    for option, value in (("--visualizer", "unknown"), ("--visual-theme", "unknown"), ("--visual-gain", "401"), ("--visual-smoothing", "100"), ("--visual-bands", "7")):
        run(option, value, "--print-config", success=False)
    explicit = folder / "portable.toml"
    default.write_text('wallpaper=true\nwallpaper_backend="plasma"\n', encoding="utf-8")
    effective = run("--print-config")
    assert 'wallpaper = true' in effective and 'wallpaper_backend = "plasma"' in effective
    assert 'wallpaper_backend = "auto"' in run("--wallpaper-backend", "auto", "--print-config")
    assert 'wallpaper = false' in run("--wallpaper", "false", "--print-config")
    run("--wallpaper-backend", "unknown", "--print-config", success=False)
    run("--config", str(explicit), "--print-config", success=False)
    run("--config", str(explicit), "--init-config")
    assert "volume = 10" in run("--config", str(explicit), "--print-config")
    default.write_text('[bindings]\npause=["q"]\n', encoding="utf-8")
    assert str(default) in run("--print-config", success=False)
    assert "volume = 10" in run("--no-config", "--print-config")
    default.write_text('volum=10\n', encoding="utf-8")
    run("--print-config", success=False)
print("Configuration CLI: paths, initialization, overrides, bindings and errors passed.")
