"""Exercise pasted terminal paths through the menu on Windows and Linux."""
import os
from pathlib import Path
import shlex
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
binary = root / "target/release" / ("asiji.exe" if os.name == "nt" else "asiji")
with tempfile.TemporaryDirectory(prefix="asiji import ") as temporary:
    base = Path(temporary)
    source, library = base / "source", base / "library"
    source.mkdir()
    library.mkdir()
    names = ["Песня с пробелами.mp3", "Песня с пробелами.mp4", "it's $(echo hello).wav", "uri track.flac"]
    for name in names:
        (source / name).write_bytes(name.encode())
    (source / "unsupported.txt").write_text("not media")
    (library / names[2]).write_bytes(b"existing file must survive")
    def quote(path):
        return '"' + str(path) + '"' if os.name == "nt" else shlex.quote(str(path))
    inputs = [
        " ".join(quote(source / name) for name in names[:3]),
        (source / names[3]).as_uri(),
        quote(library / names[0]),
        quote(source / "unsupported.txt"),
        quote(source / "missing.mp4"),
        "'unfinished",
        "r", "q",
    ]
    result = subprocess.run([str(binary), "--no-config", "--media", str(library)],
                            input="\n".join(inputs) + "\n", capture_output=True,
                            encoding="utf-8", timeout=30)
    assert result.returncode == 0, result.stdout + result.stderr
    for name in (names[0], names[1], names[3]):
        assert (library / name).read_bytes() == (source / name).read_bytes()
    assert (library / names[2]).read_bytes() == b"existing file must survive"
    assert all((source / name).exists() for name in names)
    assert len(list(library.iterdir())) == 4
    assert "[VIDEO]" in result.stdout
    assert "Уже в медиатеке:" in result.stdout
    assert "Не добавлено:" in result.stdout
    assert "Незакрытая кавычка" in result.stdout
print("Menu import: quoted paths, URI, pairing, collisions, original files and errors passed.")
