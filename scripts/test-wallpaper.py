"""Interactive Windows Explorer smoke test; uses silent synthetic audio."""
import ctypes
from ctypes import wintypes as w
import os
from pathlib import Path
import subprocess
import tempfile
from terminal_test import Terminal

if os.name != "nt":
    raise SystemExit("This test requires Windows Explorer.")
root = Path(__file__).resolve().parents[1]
user = ctypes.windll.user32
callback = ctypes.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)
user.EnumChildWindows.argtypes = [w.HWND, callback, w.LPARAM]
user.GetParent.argtypes = [w.HWND]
user.GetParent.restype = w.HWND
user.GetClassNameW.argtypes = [w.HWND, w.LPWSTR, ctypes.c_int]
user.GetWindowTextW.argtypes = [w.HWND, w.LPWSTR, ctypes.c_int]
user.IsWindowVisible.argtypes = [w.HWND]

def windows():
    found = []
    @callback
    def child(window, unused):
        title = ctypes.create_unicode_buffer(128)
        user.GetWindowTextW(window, title, 128)
        if title.value == "ASIJI Wallpaper":
            found.append(window)
        return True
    @callback
    def top(window, unused):
        user.EnumChildWindows(window, child, 0)
        return True
    user.EnumWindows(top, 0)
    return found

assert not windows(), "Close existing ASIJI wallpaper sessions before testing."
with tempfile.TemporaryDirectory(prefix="asiji-wallpaper-") as temporary:
    folder = Path(temporary)
    media = folder / "media"
    media.mkdir()
    subprocess.run([str(root / "bin/ffmpeg.exe"), "-v", "error", "-f", "lavfi", "-i",
                    "sine=frequency=330:sample_rate=48000", "-t", "30", str(media / "Test.wav")], check=True)
    terminal = Terminal([str(root / "target/release/asiji.exe"), "--no-config", "--media", str(media),
                         "--cache-dir", str(folder / "cache"), "--play", "1", "--volume", "0",
                         "--wallpaper", "true", "--mode", "blocks", "--visualizer", "orbit"])
    try:
        terminal.until("VOL 0%")
        active = windows()
        assert len(active) == 1
        parent = user.GetParent(active[0])
        name = ctypes.create_unicode_buffer(128)
        user.GetClassNameW(parent, name, 128)
        assert name.value == "WorkerW" and user.IsWindowVisible(active[0])
        terminal.process.setwinsize(1, 1)
        terminal.process.setwinsize(35, 120)
        terminal.until("00:01")
        assert windows() == active, "Wallpaper stopped on console resize"
        terminal.send(" ")
        terminal.until("PAUSE")
        terminal.send("h")
        terminal.until("ASCII")
        terminal.send("m")
        terminal.until("ЗВУК ВЫКЛЮЧЕН")
        terminal.send("q")
        terminal.until("Q — выход: ")
        assert not windows(), "Wallpaper window leaked"
        terminal.send("q\r")
        terminal.finish()
    finally:
        terminal.close()
print("Wallpaper host, resize, pause, ASCII/HD, mute indication and cleanup passed.")
