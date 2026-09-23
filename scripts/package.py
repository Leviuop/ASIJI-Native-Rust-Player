"""Package a native build using an explicit list of distributable files."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import zipfile

root = Path(__file__).resolve().parents[1]
os.chdir(root)
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--format-version", "1"
]))
package = next(p for p in metadata["packages"] if p["name"] == "asiji")
windows = os.name == "nt"
platform = "windows-x86_64" if windows else "linux-x86_64"
name = f"asiji-{package['version']}-{platform}"
staging = root / "dist" / name
staging.mkdir(parents=True, exist_ok=False)
(staging / "bin").mkdir()
(staging / "media").mkdir()
binary = "asiji.exe" if windows else "asiji"
shutil.copy2(Path(metadata["target_directory"]) / "release" / binary, staging / "bin" / binary)
for filename in ("README.md", "LICENSE", "THIRD_PARTY.md", "CHANGELOG.md", "config.example.toml"):
    shutil.copy2(root / filename, staging / filename)
shutil.copy2(root / "media" / "README.txt", staging / "media" / "README.txt")
shutil.copytree(root / "docs", staging / "docs")
if windows:
    shutil.copy2(root / "start.bat", staging / "start.bat")
    shutil.copy2(root / "install.bat", staging / "install.bat")
    (staging / "scripts").mkdir()
    shutil.copy2(root / "scripts" / "install-windows.ps1", staging / "scripts" / "install-windows.ps1")
    shutil.copy2(root / "scripts" / "setup-ffmpeg.ps1", staging / "scripts" / "setup-ffmpeg.ps1")
else:
    (staging / "scripts").mkdir()
    shutil.copy2(root / "scripts/test-linux-gpu.sh", staging / "scripts/test-linux-gpu.sh")
    shutil.copy2(root / "start.sh", staging / "start.sh")
    (staging / "start.sh").chmod(0o755)
    shutil.copy2(root / "install.sh", staging / "install.sh")
    (staging / "install.sh").chmod(0o755)
    (staging / "bin" / binary).chmod(0o755)

target = "x86_64-pc-windows-msvc" if windows else "x86_64-unknown-linux-gnu"
subprocess.run([sys.executable, str(root / "scripts/license_audit.py"),
                "--target", target, "--output", str(staging)], check=True)

if windows:
    archive = root / "dist" / f"{name}.zip"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED, strict_timestamps=False) as output:
        for source in sorted(staging.rglob("*")):
            if source.is_file():
                output.write(source, source.relative_to(staging.parent))
else:
    archive = root / "dist" / f"{name}.tar.gz"
    with tarfile.open(archive, "w:gz") as output:
        output.add(staging, arcname=name)
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
archive.with_name(archive.name + ".sha256").write_text(
    f"{digest}  {archive.name}\n", encoding="ascii"
)
print(archive)
