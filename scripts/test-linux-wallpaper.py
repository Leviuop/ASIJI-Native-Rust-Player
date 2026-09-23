"""Real desktop smoke checks inside an isolated Xvfb / D-Bus session."""
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import urllib.request
from PIL import Image, ImageGrab
from terminal_test import Terminal

root = Path(__file__).resolve().parents[1]
kind = sys.argv[1]
binary = root / "target/release/asiji"
evidence = root / "desktop-evidence"
evidence.mkdir(exist_ok=True)
children = []
terminal = None

def run(*args):
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT, timeout=15).strip()

def wait_for(check, timeout=40):
    deadline = time.monotonic() + timeout
    error = None
    while time.monotonic() < deadline:
        try:
            value = check()
            if value:
                return value
        except (OSError, subprocess.SubprocessError, ValueError) as caught:
            error = caught
        time.sleep(0.2)
    raise AssertionError(f"Desktop not ready: {error}")

def spawn(*args):
    log = open(evidence / (kind + "-" + Path(args[0]).name + ".log"), "w")
    child = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    children.append((child, log))
    return child

def qdbus(script):
    executable = next((p for p in ("qdbus6", "qdbus-qt6", "/usr/lib/qt6/bin/qdbus",
                                    "/usr/lib/qt5/bin/qdbus", "qdbus") if shutil.which(p)), None)
    assert executable, "qdbus missing"
    return run(executable, "org.kde.plasmashell", "/PlasmaShell",
               "org.kde.PlasmaShell.evaluateScript", script)

def snapshot(label):
    path = evidence / (kind + "-" + label + ".png")
    if kind == "mpvpaper":
        run("grim", str(path))
        picture = Image.open(path).convert("RGB")
    else:
        picture = ImageGrab.grab().convert("RGB")
        picture.save(path)
    w, h = picture.size
    pixels = picture.crop((w // 4, h // 4, 3 * w // 4, 3 * h // 4)).getdata()
    return sum(max(p) > 70 and max(p) - min(p) > 25 for p in pixels)

with tempfile.TemporaryDirectory(prefix="asiji-desktop-") as temporary:
    home = Path(temporary)
    runtime = home / "runtime"
    runtime.mkdir(mode=0o700)
    os.environ.update(HOME=str(home), XDG_RUNTIME_DIR=str(runtime),
                      XDG_CONFIG_HOME=str(home / "config"), XDG_DATA_HOME=str(home / "data"),
                      XDG_CACHE_HOME=str(home / "cache"),
                      ALSA_CONFIG_PATH=str(root / "tests/alsa-null.conf"))
    os.environ.pop("WAYLAND_DISPLAY", None)
    try:
        if kind == "mpvpaper":
            os.environ.update(XDG_CURRENT_DESKTOP="sway", WLR_BACKENDS="headless",
                              WLR_RENDERER="pixman", WLR_LIBINPUT_NO_DEVICES="1")
            config = home / "sway.conf"
            config.write_text("output * mode 1280x720\noutput * bg #000000 solid_color\n")
            spawn("sway", "-c", str(config))
            socket = wait_for(lambda: next((p for p in runtime.glob("wayland-*") if p.is_socket()), None))
            os.environ["WAYLAND_DISPLAY"] = socket.name
        else:
            spawn("openbox")
            if kind == "plasma":
                os.environ["XDG_CURRENT_DESKTOP"] = "KDE"
                spawn("plasmashell")
                wait_for(lambda: qdbus("print(desktops().length)") not in ("", "0"))
                qdbus("desktops().forEach(function(d) { d.wallpaperPlugin='org.kde.color';"
                      "d.currentConfigGroup=['Wallpaper','org.kde.color','General'];d.writeConfig('Color','#000000');});")
            elif kind == "gnome":
                os.environ.update(XDG_CURRENT_DESKTOP="GNOME", WAYLAND_DISPLAY="asiji-gnome")
                run(str(binary), "--no-config", "--wallpaper-backend", "gnome", "--wallpaper-setup")
                run("gsettings", "set", "org.gnome.shell", "enabled-extensions", "['asiji@leviuop.github.io']")
                run("gsettings", "set", "org.gnome.desktop.background", "picture-uri", "''")
                run("gsettings", "set", "org.gnome.desktop.background", "picture-uri-dark", "''")
                run("gsettings", "set", "org.gnome.desktop.background", "primary-color", "'#000000'")
                spawn("gnome-shell", "--nested", "--wayland", "--wayland-display=asiji-gnome")
                wait_for(lambda: (runtime / "asiji-wallpaper/gnome-ready").exists())
                run("xdotool", "key", "Escape")
            else:
                os.environ["XDG_CURRENT_DESKTOP"] = "XFCE"
                run("xsetroot", "-solid", "black")
        time.sleep(1)
        baseline = snapshot("before")
        media = home / "media"
        media.mkdir()
        run("ffmpeg", "-v", "error", "-f", "lavfi", "-i",
            "sine=frequency=330:sample_rate=48000", "-t", "40", str(media / "Test.wav"))
        terminal = Terminal([str(binary), "--no-config", "--media", str(media), "--cache-dir", str(home / "audio-cache"),
                             "--play", "1", "--volume", "0", "--mode", "blocks", "--width", "80",
                             "--wallpaper", "true", "--wallpaper-backend", kind])
        terminal.until("VOL 0%")
        time.sleep(3)
        if kind == "plasma":
            assert qdbus("print(desktops()[0].wallpaperPlugin)") == "io.github.Leviuop.asiji"
            url = qdbus("var d=desktops()[0];d.currentConfigGroup=['Wallpaper','io.github.Leviuop.asiji','General'];print(d.readConfig('SourceUrl'));")
        elif kind == "gnome":
            url = json.loads((runtime / "asiji-wallpaper/gnome.json").read_text())["url"]
        else:
            url = None
        if url:
            frame = urllib.request.urlopen(url + "/frame.bmp", timeout=3).read()
            assert frame[:2] == b"BM" and len(frame) > 1000
        colored = snapshot("blocks")
        assert colored > baseline + 200, (kind, baseline, colored)
        terminal.send(" ")
        terminal.until("PAUSE")
        terminal.send("h")
        terminal.until("ASCII")
        time.sleep(1)
        assert snapshot("ascii") > baseline + 50
        terminal.send("q")
        terminal.until("Q — выход: ")
        time.sleep(1)
        if kind == "plasma":
            assert qdbus("print(desktops()[0].wallpaperPlugin)") == "org.kde.color"
        elif kind == "gnome":
            assert not (runtime / "asiji-wallpaper/gnome.json").exists()
        assert snapshot("after") <= baseline + 200
        terminal.send("q\n")
        terminal.finish()
        print(f"{kind}: actual desktop pixels, pause, ASCII/HD and background restoration passed")
    finally:
        if terminal:
            terminal.close()
        for child, log in reversed(children):
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
            log.close()
