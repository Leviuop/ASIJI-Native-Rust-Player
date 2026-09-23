"""Diagnose real video and absent dependencies without a terminal or audio output."""
import os
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
binary = root / "target/release" / ("asiji.exe" if os.name == "nt" else "asiji")
ffmpeg = str(root / "bin/ffmpeg.exe") if os.name == "nt" else "ffmpeg"

def run(*arguments, environment=None, success=True):
    result = subprocess.run([str(binary), "--no-config", "--doctor", *arguments],
                            capture_output=True, encoding="utf-8", env=environment, timeout=90)
    output = result.stdout + result.stderr
    assert (result.returncode == 0) == success, output
    return output

assert "FFmpeg" in run()
run(environment=dict(os.environ, ASIJI_FFMPEG="asiji-nonexistent-ffmpeg"), success=False)
with tempfile.TemporaryDirectory() as temporary:
    video = Path(temporary) / "diagnostic clip.mp4"
    subprocess.run([ffmpeg, "-v", "error", "-f", "lavfi", "-i", "testsrc2=s=160x90:r=24",
                    "-t", "1", "-c:v", "libx264", str(video)], check=True, timeout=30)
    output = run("--doctor-video", str(video))
    assert "CPU: OK" in output and "CUDA:" in output, output
    output = run("--doctor-video", str(video), "--hwaccel-device", "999999")
    assert "CPU: OK" in output and "CUDA: FAIL" in output, output
    run("--doctor-video", str(video.with_name("missing.mp4")), success=False)
print("Doctor: dependencies, real decoding, unavailable GPU and invalid file passed.")
