"""Real terminal playback checks; Linux CI uses ALSA null."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
from terminal_test import Terminal
root = Path(__file__).resolve().parents[1]
windows = os.name == "nt"
binary = root / "target/release" / ("asiji.exe" if windows else "asiji")
ffmpeg = str(root / "bin/ffmpeg.exe") if windows else "ffmpeg"

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
