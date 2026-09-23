"""Exercise release notice collection and fail-closed license policy."""
import json
from pathlib import Path
import subprocess
import tempfile
from unittest.mock import patch

import license_audit

host = next(line[6:] for line in subprocess.check_output(["rustc", "-vV"], text=True).splitlines()
            if line.startswith("host: "))
with tempfile.TemporaryDirectory(prefix="asiji-notices-") as temporary:
    folder = Path(temporary)
    license_audit.collect(host, folder)
    report = json.loads((folder / "licenses/DEPENDENCIES.json").read_text())
    packages = {p["name"]: p for p in report["packages"]}
    assert "dasp_sample" in packages and "symphonia" in packages
    assert not any(name in packages for name in ("ndk", "objc2", "cesu8"))
    assert len(list((folder / "sources").glob("*.crate"))) == 5
    assert (folder / "licenses/rust-standard-library/COPYRIGHT-library.html").is_file()
    for p in packages.values():
        assert p["notices"]
        assert all((folder / notice).exists() for notice in p["notices"])
        if p["license_expression"] == "MPL-2.0":
            assert p["source_sha256"] and (folder / p["source_archive"]).is_file()
    with patch.object(license_audit, "REVIEWED", set()):
        try:
            license_audit.collect(host, folder / "blocked")
        except RuntimeError as error:
            assert "Unreviewed license" in str(error)
        else:
            raise AssertionError("Unreviewed license did not block packaging")
    with patch.object(license_audit, "notice_files", return_value=[]):
        try:
            license_audit.collect(host, folder / "missing")
        except RuntimeError as error:
            assert "Missing license notice" in str(error)
        else:
            raise AssertionError("Missing notices did not block packaging")
    try:
        license_audit.notice_files(folder, "../outside-LICENSE")
    except RuntimeError:
        pass
    else:
        raise AssertionError("Out-of-package license path accepted")
print("Release license collection, platform scope, source integrity and rejection checks passed.")
