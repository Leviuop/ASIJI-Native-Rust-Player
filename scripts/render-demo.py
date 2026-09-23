"""Compose README artwork from actual native visualizer frames (synthetic audio)."""
from pathlib import Path
import os
import subprocess
from PIL import Image, ImageDraw, ImageFont

root = Path(__file__).resolve().parents[1]
frames = root / ".tools/visualizer-demo"
subprocess.run(["cargo", "test", "--release", "--locked", "export_visualizer_demo", "--", "--ignored"],
               cwd=root, env=dict(os.environ, ASIJI_DEMO_DIR=str(frames)), check=True)
output = root / "docs/assets"
output.mkdir(parents=True, exist_ok=True)
font_path = next((p for p in [Path("C:/Windows/Fonts/consola.ttf"), Path("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf")] if p.exists()), None)
font = ImageFont.truetype(str(font_path), 16) if font_path else ImageFont.load_default()
small = ImageFont.truetype(str(font_path), 12) if font_path else font
large = ImageFont.truetype(str(font_path), 48) if font_path else font
animation = []
for index in range(60):
    canvas = Image.new("RGB", (1080, 370), "#080d14")
    draw = ImageDraw.Draw(canvas)
    draw.text((34, 21), "ASIJI", font=large, fill="#e8f5f6")
    draw.text((260, 31), "MUSIC IN CHARACTERS", font=font, fill="#40e6bb")
    draw.text((260, 58), "Native Rust  /  Windows + Linux  /  Your terminal", font=small, fill="#93a5b7")
    for tile, (mode, theme, accent) in enumerate([("bars", "AURORA", "#40e6bb"), ("wave", "ICE", "#a7d8ff"), ("orbit", "EMBER", "#ff8666")]):
        x = 28 + tile * 350
        draw.rounded_rectangle((x, 112, x+324, 318), radius=8, fill="#0d151f", outline="#243342")
        draw.text((x+12, 121), f"{mode.upper()}  /  {theme}", font=small, fill=accent)
        frame = Image.open(frames / f"{mode}-{index:03}.ppm").convert("RGB")
        frame = frame.resize((320,160), Image.Resampling.NEAREST)
        canvas.paste(frame,(x+2,151))
    draw.text((32, 342), "REAL RENDER FRAMES / SYNTHETIC AUDIO     FFT 2048 / LOG BANDS / STEREO", font=small, fill="#7c90a5")
    animation.append(canvas)
animation[20].save(output / "visualizer.png")
animation[0].save(output / "visualizer.gif", save_all=True, append_images=animation[1:], duration=83, loop=0, optimize=True)
print(output / "visualizer.gif")
