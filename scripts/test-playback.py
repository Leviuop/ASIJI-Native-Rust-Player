"""Real terminal playback checks; Linux CI uses ALSA null, Windows needs an output device."""
import json
import os
from pathlib import Path
import queue
import subprocess
import tempfile
import threading
import time

root = Path(__file__).resolve().parents[1]
windows = os.name == "nt"
binary = root / "target/release" / ("asiji.exe" if windows else "asiji")
ffmpeg = str(root / "bin/ffmpeg.exe") if windows else "ffmpeg"

class Terminal:
    def __init__(self, command):
        self.chunks = queue.Queue()
        if windows:
            from winpty import PtyProcess
            self.process = PtyProcess.spawn(command, dimensions=(35, 120))
        else:
            import pty
            import fcntl
            import struct
            import termios
            self.pid, self.fd = pty.fork()
            if self.pid == 0:
                fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", 35, 120, 0, 0))
                os.execv(command[0], command)
        threading.Thread(target=self.reader, daemon=True).start()

    def reader(self):
        try:
            while True:
                part = self.process.read(65536) if windows else os.read(self.fd, 65536).decode("utf-8", errors="replace")
                if not part:
                    return
                self.chunks.put(part)
        except (EOFError, OSError):
            pass

    def send(self, text):
        if windows:
            self.process.write(text)
        else:
            os.write(self.fd, text.encode())

    def until(self, marker):
        text = ""
        deadline = time.monotonic() + 20
        while marker not in text and time.monotonic() < deadline:
            try:
                text += self.chunks.get(timeout=0.1)
            except queue.Empty:
                pass
        assert marker in text, f"Missing {marker!r}: {text[-3000:]}"
        return text

    def finish(self):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if windows:
                if not self.process.isalive():
                    return
            else:
                done, status = os.waitpid(self.pid, os.WNOHANG)
                if done:
                    self.pid = None
                    assert os.waitstatus_to_exitcode(status) == 0
                    return
            time.sleep(0.05)
        raise AssertionError("Player did not exit")

    def close(self):
        if windows:
            if self.process.isalive():
                self.process.terminate(force=True)
        else:
            import signal
            if self.pid is not None:
                os.kill(self.pid, signal.SIGKILL)
                os.waitpid(self.pid, 0)
            os.close(self.fd)

with tempfile.TemporaryDirectory(prefix="asiji playback ") as temporary:
    folder = Path(temporary)
    for style in ("bars", "wave", "orbit", "video"):
        media = folder / style
        media.mkdir()
        command = [ffmpeg, "-v", "error", "-f", "lavfi", "-i", "sine=frequency=330:sample_rate=48000"]
        if style == "video":
            command += ["-f", "lavfi", "-i", "testsrc2=s=320x180:r=24", "-c:v", "libx264", "-c:a", "aac", "-t", "12", str(media / "Test.mp4")]
        else:
            command += ["-t", "12", str(media / "Test.wav")]
        subprocess.run(command, check=True, timeout=30)
        profile = folder / (style + ".json")
        terminal = Terminal([str(binary), "--no-config", "--media", str(media), "--cache-dir", str(folder / "cache"),
                             "--play", "1", "--mode", "blocks", "--volume", "0", "--width", "80", "--hwaccel", "cpu",
                             "--visualizer", style if style != "video" else "bars", "--profile", str(profile)])
        try:
            output = terminal.until("VOL 0%")
            if style != "video":
                assert style.capitalize() in output, output[-3000:]
            terminal.send(" ")
            terminal.until("PAUSE")
            terminal.send("h")
            terminal.until("ASCII")
            terminal.send("d")
            terminal.until("00:05")
            terminal.send("a")
            terminal.until("00:00")
            terminal.send(" ")
            terminal.until("PLAY")
            terminal.send("q")
            terminal.until("Q — выход: ")
            terminal.send("q\r" if windows else "q\n")
            terminal.finish()
            report = json.loads(profile.read_text())
            assert report["frames"] > 0, report
            print(f"{style}: play, pause, seek, ASCII/HD and menu passed")
        finally:
            terminal.close()
