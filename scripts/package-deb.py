"""Build a Debian package from the already audited portable distribution."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

root = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"], cwd=root))
version = metadata["packages"][0]["version"]
portable = root / "dist" / f"asiji-{version}-linux-x86_64"
staging = root / "dist" / f"asiji-{version}-deb"
(staging / "DEBIAN").mkdir(parents=True, exist_ok=False)
(staging / "usr/bin").mkdir(parents=True)
shutil.copy2(portable / "bin/asiji", staging / "usr/bin/asiji")
docs = staging / "usr/share/doc/asiji"
docs.mkdir(parents=True)
for name in ("README.md", "CHANGELOG.md", "LICENSE", "THIRD_PARTY.md", "config.example.toml"):
    shutil.copy2(portable / name, docs / name)
for name in ("licenses", "sources", "docs"):
    shutil.copytree(portable / name, docs / name)
shutil.copy2(root / "docs/HARDWARE.md", docs / "HARDWARE.md")
shutil.copy2(root / "scripts/test-linux-gpu.sh", docs / "test-linux-gpu.sh")
control = f"""Package: asiji
Version: {version}
Section: sound
Priority: optional
Architecture: amd64
Maintainer: Leviuop <Leviuop@users.noreply.github.com>
Depends: libc6 (>= 2.35), libgcc-s1, libasound2 | libasound2t64, ffmpeg
Homepage: https://github.com/Leviuop/ASIJI-Native-Rust-Player
Description: Music and video rendered as characters in a terminal
 Local audio and video player with ASCII and half-block output,
 configurable shortcuts and optional GPU video decoding.
"""
(staging / "DEBIAN/control").write_text(control, encoding="utf-8")
for path in staging.rglob("*"):
    path.chmod(0o755 if path.is_dir() or path == staging / "usr/bin/asiji" else 0o644)
archive = root / "dist" / f"asiji_{version}_amd64.deb"
subprocess.run(["dpkg-deb", "--root-owner-group", "--build", str(staging), str(archive)], check=True)
archive.with_name(archive.name + ".sha256").write_text(f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n", encoding="ascii")
print(archive)
