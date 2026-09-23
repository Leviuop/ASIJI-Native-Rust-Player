"""Select visualizer and palette using real arrow-key events, save and cancel."""
import os
from pathlib import Path
import tempfile
from terminal_test import Terminal

root = Path(__file__).resolve().parents[1]
binary = root / "target/release" / ("asiji.exe" if os.name == "nt" else "asiji")
newline = "\r" if os.name == "nt" else "\n"
with tempfile.TemporaryDirectory(prefix="asiji settings ") as directory:
    folder = Path(directory)
    config = folder / "config.toml"
    config.write_text('volume=10\n', encoding="utf-8")
    terminal = Terminal([str(binary), "--config", str(config), "--media", str(folder / "media")])
    try:
        terminal.until("Q — выход: ")
        terminal.send("s" + newline)
        terminal.until("< bars >")
        terminal.send("\x1b[D")
        terminal.until("< orbit >")
        terminal.send("\x1b[B")
        terminal.until("> Палитра")
        terminal.send("\r")  # Enter in raw mode; LF is Ctrl+J on Unix.
        terminal.until("< ember >")
        terminal.send("w")
        terminal.until("Q — выход: ")
        text = config.read_text(encoding="utf-8")
        assert 'visualizer = "orbit"' in text and 'visual_theme = "ember"' in text, text
        terminal.send("s" + newline)
        terminal.until("< orbit >")
        terminal.send("\x1b[C")
        terminal.until("< bars >")
        terminal.send("q")
        terminal.until("Q — выход: ")
        assert config.read_text(encoding="utf-8") == text
        terminal.send("q" + newline)
        terminal.finish()
    finally:
        terminal.close()
print("Interactive settings: arrows, Enter, save and cancel passed.")
