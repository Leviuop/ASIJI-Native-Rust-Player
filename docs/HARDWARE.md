# Hardware validation

CI checks CPU decoding and recovery when GPU acceleration is unavailable. It
does not certify a GPU driver or prove that VAAPI works on physical hardware.

Windows: NVIDIA RTX 5070 Ti passed CUDA and D3D11VA; integrated AMD Radeon Graphics (1002:13c0), adapter 1, decoded the synthetic H.264 clip with D3D11VA.

Native Linux AMD/Intel validation is **pending**. No such test machine is
available to the maintainer for this release. Results must not be inferred
from Windows D3D11VA or a Linux VM without GPU access.

On a Linux AMD/Intel machine with FFmpeg, a VAAPI driver and ASIJI installed:

```bash
ASIJI_PLAYER=./bin/asiji bash scripts/test-linux-gpu.sh /dev/dri/renderD128
```

The script creates a synthetic H.264 clip, lists hardware/driver details,
tests CPU and GPU decoding, then requires successful VAAPI decoding with
fallback disabled. Nonzero exit means the hardware test did not pass. Nothing
is uploaded. Review `asiji-gpu-report.txt` before attaching it to an issue.

For a complete manual check, also play a local clip in ASCII and blocks mode,
pause, seek, resize the window and return to the menu. Record:

- ASIJI version, distribution, kernel, terminal and font size;
- GPU PCI ID, driver/Mesa version, FFmpeg version and DRM device;
- decoder shown in the player, source codec/resolution and render dimensions;
- synchronization, dropped frames, recovery and error messages.

One H.264 pass does not certify HEVC, AV1, HDR or all GPUs of that vendor.
