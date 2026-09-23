"""Check settings menu persistence and cache commands without audio playback."""
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
binary = root / "target/release" / ("asiji.exe" if os.name == "nt" else "asiji")
with tempfile.TemporaryDirectory() as temporary:
    folder = Path(temporary)
    config = folder / "config.toml"
    cache = folder / "cache"
    cache.mkdir()
    config.write_text('volume=10\n', encoding="utf-8")
    result = subprocess.run([str(binary), "--config", str(config), "--media", str(folder / "media")],
        input="s\nvolume=37\nvolume=999\nmode=blocks\nbind pause=k\nimport_mode=move\nimport_conflict=rename\nw\nq\n",
        capture_output=True, encoding="utf-8", timeout=20)
    assert result.returncode == 0, result.stdout + result.stderr
    text = config.read_text(encoding="utf-8")
    assert 'volume = 37' in text and 'pause = ["k"]' in text and 'import_mode = "move"' in text
    assert config.with_suffix(".toml.bak").read_text() == 'volume=10\n'
    before = config.read_bytes()
    subprocess.run([str(binary), "--config", str(config)], input="s\nvolume=5\nq\nq\n",
                   capture_output=True, encoding="utf-8", timeout=20, check=True)
    assert config.read_bytes() == before
    (cache / ("rust-pcm-" + "a" * 64 + ".wav")).write_bytes(b"cached")
    (cache / "personal.wav").write_bytes(b"keep")
    subprocess.run([str(binary), "--no-config", "--cache-dir", str(cache), "--clear-cache"], check=True, timeout=10)
    assert sorted(p.name for p in cache.iterdir()) == [".lock", "personal.wav"]
print("Settings menu: save, cancel, validation, bindings, backup and cache clear passed.")
