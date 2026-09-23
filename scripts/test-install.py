"""Exercise per-user installation in temporary directories, preserving user files."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
windows = os.name == "nt"
binary = "asiji.exe" if windows else "asiji"
with tempfile.TemporaryDirectory(prefix="asiji install ") as temporary:
    base = Path(temporary)
    source, destination, links = base / "release", base / "installed", base / "commands"
    source.mkdir()
    for folder in ("bin", "scripts", "media"):
        (source / folder).mkdir()
    shutil.copy2(root / "target/release" / binary, source / "bin" / binary)
    for name in ("README.md", "LICENSE", "THIRD_PARTY.md", "CHANGELOG.md", "config.example.toml",
                 "start.bat", "install.bat", "start.sh", "install.sh"):
        shutil.copy2(root / name, source / name)
    for name in ("install-windows.ps1", "setup-ffmpeg.ps1"):
        shutil.copy2(root / "scripts" / name, source / "scripts" / name)
    shutil.copy2(root / "media/README.txt", source / "media/README.txt")
    (source / "media/private.mp4").write_bytes(b"must not copy")
    (destination / "media").mkdir(parents=True)
    (destination / "media/keep.mp4").write_bytes(b"user media")
    (destination / "config.toml").write_text("volume=37")
    for folder in ("licenses", "sources"):
        (source / folder).mkdir()
        (source / folder / "notice.txt").write_text("preserve distribution notices")
    environment = dict(os.environ, ASIJI_BIN_DIR=str(links))
    command = (["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
                str(source / "scripts/install-windows.ps1"), "-Destination", str(destination), "-NoShortcut"]
               if windows else ["bash", str(source / "install.sh"), str(destination)])
    for _ in range(2):
        subprocess.run(command, env=environment, check=True, timeout=30)
    assert (destination / "media/keep.mp4").read_bytes() == b"user media"
    assert (destination / "config.toml").read_text() == "volume=37"
    assert not (destination / "media/private.mp4").exists()
    for folder in ("licenses", "sources"):
        assert (destination / folder / "notice.txt").is_file()
    executable = destination / "bin" / binary if windows else links / "asiji"
    result = subprocess.run([str(executable), "--no-config", "--print-config"],
                            check=True, capture_output=True, text=True, timeout=10)
    assert "volume = 10" in result.stdout
print("Installation and repeat installation preserve media, config and notices.")
