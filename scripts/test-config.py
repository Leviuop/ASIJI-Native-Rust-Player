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
    explicit = folder / "portable.toml"
    run("--config", str(explicit), "--print-config", success=False)
    run("--config", str(explicit), "--init-config")
    assert "volume = 10" in run("--config", str(explicit), "--print-config")
    default.write_text('[bindings]\npause=["q"]\n', encoding="utf-8")
    assert str(default) in run("--print-config", success=False)
    assert "volume = 10" in run("--no-config", "--print-config")
    default.write_text('volum=10\n', encoding="utf-8")
    run("--print-config", success=False)
print("Configuration CLI: paths, initialization, overrides, bindings and errors passed.")
