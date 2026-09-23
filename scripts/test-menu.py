"""Exercise refresh and playback errors in a real Linux pseudoterminal."""

import errno
import os
from pathlib import Path
import pty
import select
import shlex
import signal
import tempfile
import time

root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as temporary:
    Path(temporary, "broken.mp4").write_bytes(b"not a video")
    incoming = Path(temporary, "incoming")
    incoming.mkdir()
    dropped = incoming / "Drop with spaces.mp3"
    dropped.write_bytes(b"import without playback")
    pid, terminal = pty.fork()
    if pid == 0:
        os.execv(str(root / "target/release/asiji"), [
            "asiji", "--media", temporary, "--hwaccel", "cpu"
        ])

    def read_until(marker):
        output = bytearray()
        deadline = time.monotonic() + 20
        while marker not in output and time.monotonic() < deadline:
            if select.select([terminal], [], [], 0.2)[0]:
                try:
                    chunk = os.read(terminal, 65536)
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
                if not chunk:
                    break
                output.extend(chunk)
        assert marker in output, output.decode("utf-8", errors="replace")
        return bytes(output)

    try:
        prompt = "Q — выход: ".encode()
        for command in (None, b"r\n", b"r\n"):
            if command:
                os.write(terminal, command)
            output = read_until(prompt)
            assert b"\x1b[2J" in output and b"\x1b[3J" in output
            assert output.count(b"MUSIC IN CHARACTERS") == 1
        os.write(terminal, (shlex.quote(str(dropped)) + "\n").encode())
        output = read_until(prompt)
        assert "Добавлено:".encode() in output
        assert Path(temporary, dropped.name).read_bytes() == dropped.read_bytes()
        os.write(terminal, b"1\n")
        output = read_until(prompt)
        clear = output.rfind(b"\x1b[2J")
        assert clear >= 0
        assert "Ошибка:".encode() in output[clear:]
        os.write(terminal, b"q\n")
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            done, status = os.waitpid(pid, os.WNOHANG)
            if done:
                assert os.waitstatus_to_exitcode(status) == 0
                pid = None
                break
            time.sleep(0.05)
        assert pid is None, "Menu did not exit"
        print("Menu import, refresh, scrollback clearing and error recovery passed.")
    finally:
        os.close(terminal)
        if pid is not None:
            os.kill(pid, signal.SIGKILL)
            os.waitpid(pid, 0)
