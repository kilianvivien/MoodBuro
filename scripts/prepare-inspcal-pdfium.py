#!/usr/bin/env python3
"""Prepare the pinned, verified PDFium static library for the InspCal engine.

    python3 scripts/prepare-inspcal-pdfium.py            # download + verify
    python3 scripts/prepare-inspcal-pdfium.py --archive macos.zip   # offline
    python3 scripts/prepare-inspcal-pdfium.py --check    # verify only

Everything is pinned by assets/inspcal/pdfium-manifest.json: the archive hash,
the single archive member extracted, the final library hash, its architectures
and minimum macOS version. The result lands in .inspcal-pdfium/lib/libpdfium.a,
which .cargo/config.toml hands to pdfium-render's build script. Cargo itself
never touches the network for PDFium.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "assets" / "inspcal" / "pdfium-manifest.json"
OUTPUT = ROOT / ".inspcal-pdfium" / "lib" / "libpdfium.a"
STAMP = OUTPUT.with_suffix(".json")


def sha256(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            value.update(block)
    return value.hexdigest()


def fail(message: str) -> None:
    raise SystemExit(f"prepare-inspcal-pdfium: {message}")


def verify_library(path: Path, manifest: dict) -> None:
    library = manifest["library"]
    if sha256(path) != library["sha256"]:
        fail(f"{path} does not match the manifest hash")
    if platform.system() != "Darwin":
        return
    arches = subprocess.check_output(["lipo", "-archs", str(path)], text=True).split()
    if sorted(arches) != sorted(library["architectures"]):
        fail(f"architectures {arches} != {library['architectures']}")
    with tempfile.TemporaryDirectory() as scratch:
        for arch in arches:
            thin = Path(scratch) / f"{arch}.a"
            subprocess.check_call(["lipo", str(path), "-thin", arch, "-output", str(thin)])
            if sha256(thin) != library["slices_sha256"][arch]:
                fail(f"{arch} slice does not match the manifest hash")
            # Every member must target the recorded minimum macOS or older.
            listing = subprocess.check_output(["otool", "-l", str(thin)], text=True, stderr=subprocess.DEVNULL)
            minimums = {line.split()[1] for line in listing.splitlines() if line.strip().startswith("minos ")}
            newest = max(minimums, key=lambda value: tuple(int(part) for part in value.split(".")), default=None)
            if newest is None or tuple(map(int, newest.split("."))) > tuple(map(int, library["minimum_macos"].split("."))):
                fail(f"{arch} slice requires macOS {newest}, manifest says {library['minimum_macos']}")


def extract(archive: Path, manifest: dict) -> None:
    if sha256(archive) != manifest["archive"]["sha256"]:
        fail(f"{archive} does not match the manifest archive hash")
    member = manifest["member"]
    with zipfile.ZipFile(archive) as bundle:
        info = bundle.getinfo(member)
        mode = (info.external_attr >> 16) & 0o170000
        if mode and mode != 0o100000:
            fail(f"{member} is not a regular file in the archive")
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        partial = OUTPUT.with_suffix(".partial")
        with bundle.open(info) as source, partial.open("wb") as target:
            shutil.copyfileobj(source, target)
    verify_library(partial, manifest)
    os.replace(partial, OUTPUT)
    STAMP.write_text(json.dumps({"manifest_version": manifest["version"], "sha256": manifest["library"]["sha256"]}) + "\n")


def download(manifest: dict, directory: Path) -> Path:
    url = manifest["archive"]["url"]
    if not url.startswith("https://github.com/"):
        fail("archive URL must be an https GitHub release asset")
    target = directory / "pdfium.zip"
    print(f"downloading {url}")
    with urllib.request.urlopen(url, timeout=120) as response, target.open("wb") as handle:
        limit = manifest["archive"]["size"]
        copied = 0
        while block := response.read(1 << 20):
            copied += len(block)
            if copied > limit:
                fail("archive is larger than the manifest says")
            handle.write(block)
    return target


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--archive", type=Path, help="use an already downloaded archive")
    parser.add_argument("--check", action="store_true", help="verify the prepared library only")
    args = parser.parse_args()
    manifest = json.loads(MANIFEST.read_text())
    if args.check:
        if not OUTPUT.exists():
            fail(f"{OUTPUT.relative_to(ROOT)} is missing; run this script without --check")
        verify_library(OUTPUT, manifest)
        print(f"{OUTPUT.relative_to(ROOT)} verified (PDFium {manifest['release']})")
        return 0
    if OUTPUT.exists():
        try:
            verify_library(OUTPUT, manifest)
            print(f"{OUTPUT.relative_to(ROOT)} already prepared (PDFium {manifest['release']})")
            return 0
        except SystemExit as error:
            print(f"{error}; preparing again", file=sys.stderr)
    with tempfile.TemporaryDirectory() as scratch:
        archive = args.archive or download(manifest, Path(scratch))
        extract(archive, manifest)
    print(f"prepared {OUTPUT.relative_to(ROOT)} (PDFium {manifest['release']})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
