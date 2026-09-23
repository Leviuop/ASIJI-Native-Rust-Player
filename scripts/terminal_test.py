"""Shared pseudoterminal driver for interactive player tests."""
import codecs
import os
import queue
import threading
import time

windows = os.name == "nt"

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
        decoder = codecs.getincrementaldecoder("utf-8")(errors="replace")
        try:
            while True:
                raw = self.process.read(65536) if windows else os.read(self.fd, 65536)
                if not raw:
                    return
                self.chunks.put(raw if windows else decoder.decode(raw))
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

