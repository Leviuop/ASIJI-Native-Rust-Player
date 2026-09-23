"""Check the release dependency graph and collect notices from pinned crate sources."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
# Exact expressions reviewed for this release. New terms require a human review.
REVIEWED = {
    "MIT", "Apache-2.0", "MIT OR Apache-2.0", "Apache-2.0 OR MIT",
    "MIT/Apache-2.0", "Apache-2.0/MIT", "Unlicense OR MIT",
    "Zlib OR Apache-2.0 OR MIT", "MPL-2.0",
    "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT",
    "(Apache-2.0 OR MIT) AND BSD-3-Clause",
    "(MIT OR Apache-2.0) AND Unicode-3.0",
}
MPL = {f"{name}-0.5.5" for name in (
    "symphonia", "symphonia-core", "symphonia-codec-pcm",
    "symphonia-format-riff", "symphonia-metadata")}


def active_packages(metadata):
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    pending = [metadata["resolve"]["root"]]
    seen = set()
    while pending:
        key = pending.pop()
        if key not in seen:
            seen.add(key)
            pending.extend(nodes[key]["dependencies"])
    return sorted((p for p in metadata["packages"] if p["id"] in seen and p["name"] != "asiji"),
                  key=lambda p: (p["name"], p["version"]))


def notice_files(folder, license_file=None):
    result = {p for p in folder.iterdir()
              if p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))}
    if license_file:
        path = (folder / license_file).resolve()
        if not path.is_relative_to(folder.resolve()) or not path.is_file():
            raise RuntimeError(f"Invalid license_file: {path}")
        result.add(path)
    return sorted(result)


def collect(target, output):
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", target
    ], cwd=ROOT))
    notices, sources = output / "licenses", output / "sources"
    notices.mkdir(parents=True, exist_ok=True)
    sources.mkdir(parents=True, exist_ok=True)
    shutil.copytree(ROOT / "licenses", notices, dirs_exist_ok=True)
    inventory = []
    for package in active_packages(metadata):
        label = f"{package['name']}-{package['version']}"
        expression = package.get("license")
        if expression not in REVIEWED:
            raise RuntimeError(f"Unreviewed license: {label}: {expression}")
        folder = Path(package["manifest_path"]).parent
        files = notice_files(folder, package.get("license_file"))
        supplemental = []
        if label == "dasp_sample-0.11.0":
            supplemental = [ROOT / "licenses/dasp_sample-LICENSE-MIT.txt"]
        if label in MPL:
            supplemental = [ROOT / "licenses/MPL-2.0.txt"]
        if not files and not supplemental:
            raise RuntimeError(f"Missing license notice: {label}")
        for name in ({"LICENSE-UNICODE"} if "AND Unicode-3.0" in expression else set()) | (
                {"LICENSE-WHATWG"} if label.startswith("encoding_rs-") else set()):
            if not (folder / name).is_file():
                raise RuntimeError(f"Missing additional notice: {label}/{name}")
        copied = []
        for source in files + supplemental:
            destination = notices / label / source.name
            destination.parent.mkdir(parents=True, exist_ok=True)
            if source.is_dir():
                shutil.copytree(source, destination, dirs_exist_ok=True)
            else:
                shutil.copy2(source, destination)
            copied.append(destination.relative_to(output).as_posix())
        source_archive = None
        checksum = None
        if expression == "MPL-2.0":
            if label not in MPL:
                raise RuntimeError(f"New MPL component needs source review: {label}")
            original = folder.parents[2] / "cache" / folder.parent.name / f"{label}.crate"
            checksum = hashlib.sha256(original.read_bytes()).hexdigest()
            # Cargo's lockfile pins the published source archive, including its notices.
            lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
            block = next((b for b in lock.split("[[package]]") if
                          f'name = "{package["name"]}"' in b and
                          f'version = "{package["version"]}"' in b), "")
            if f'checksum = "{checksum}"' not in block:
                raise RuntimeError(f"MPL source checksum mismatch: {label}")
            with tarfile.open(original) as archive:
                for member in archive.getmembers():
                    if member.isfile():
                        relative = Path(*Path(member.name).parts[1:])
                        local = (folder / relative).resolve()
                        if not local.is_relative_to(folder.resolve()) or local.read_bytes() != archive.extractfile(member).read():
                            raise RuntimeError(f"Modified MPL source: {label}/{relative}")
            destination = sources / original.name
            shutil.copy2(original, destination)
            source_archive = destination.relative_to(output).as_posix()
        inventory.append({"name": package["name"], "version": package["version"],
                          "license_expression": expression, "repository": package.get("repository"),
                          "notices": copied, "source_archive": source_archive, "source_sha256": checksum})
    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
    rust_docs = sysroot / "share/doc/rust"
    copyright_file = rust_docs / "COPYRIGHT-library.html"
    if not copyright_file.is_file() or not (rust_docs / "licenses").is_dir():
        raise RuntimeError("Rust standard-library notices missing; run: rustup component add rust-docs")
    rust_notices = notices / "rust-standard-library"
    rust_notices.mkdir(exist_ok=True)
    shutil.copy2(copyright_file, rust_notices / copyright_file.name)
    shutil.copytree(rust_docs / "licenses", rust_notices / "licenses", dirs_exist_ok=True)
    rust_version = subprocess.check_output(["rustc", "--version", "--verbose"], text=True)
    (rust_notices / "BUILD-INFO.txt").write_text(rust_version + f"\nASIJI target: {target}\n", encoding="utf-8")
    report = {"target": target, "scope": "Resolved Cargo dependencies, including build tools; not a binary symbol inventory",
              "rustc": rust_version, "packages": inventory}
    (notices / "DEPENDENCIES.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    (notices / "INDEX.txt").write_text("\n".join(
        f"{p['name']}-{p['version']}: {p['license_expression']}" for p in inventory) + "\n", encoding="utf-8")
    print(f"{target}: {len(inventory)} dependencies checked; notices, MPL source integrity and Rust runtime notices collected")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    host = re.search(r"^host: (.+)$", subprocess.check_output(["rustc", "-vV"], text=True), re.M).group(1)
    if args.output:
        collect(args.target or host, args.output)
    else:
        with tempfile.TemporaryDirectory(prefix="asiji-licenses-") as directory:
            collect(args.target or host, Path(directory))
