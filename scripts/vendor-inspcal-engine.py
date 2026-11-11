#!/usr/bin/env python3
"""Vendor a committed InspCal engine snapshot into vendor/inspcal.

InspCal is a private repository and MoodBuro is public, so MoodBuro carries a
copy of the MIT-licensed engine source plus InspCal's *fictional* public
fixtures and reviewed goldens, never a submodule. Private timetable PDFs and
their goldens are not tracked upstream and are never copied.

    python3 scripts/vendor-inspcal-engine.py --source ../InspCal [--commit SHA]
    python3 scripts/vendor-inspcal-engine.py --check

The snapshot is taken with `git archive` from a commit, so uncommitted upstream
edits cannot leak in. `--check` re-hashes vendor/inspcal against
vendor/inspcal-provenance.json and fails on any drift.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import shutil
import subprocess
import sys
import re
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / "vendor" / "inspcal"
PROVENANCE = ROOT / "vendor" / "inspcal-provenance.json"
PDFIUM_MANIFEST = ROOT / "assets" / "inspcal" / "pdfium-manifest.json"
# Bump when md-inspcal's expectations of the engine change (see crates/md-inspcal).
ADAPTER_CONTRACT = 1
UPSTREAM = "kilianvivien/InspCal"
PATHS = ["LICENSE", "engine", "tests/fixtures/public", "tests/golden/public"]
# Engine binaries, build output and anything else outside this list are skipped.
SKIP = {"engine/target"}


def sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def tree_digest(files: dict[str, str], prefix: str) -> str:
    value = hashlib.sha256()
    for name in sorted(files):
        if name.startswith(prefix):
            value.update(name.encode() + b"\0" + files[name].encode() + b"\n")
    return value.hexdigest()


def snapshot_files() -> dict[str, str]:
    files = {}
    for path in sorted(DEST.rglob("*")):
        if path.is_symlink():
            raise SystemExit(f"symlink in vendored tree: {path}")
        if path.is_file():
            files[path.relative_to(DEST).as_posix()] = sha256(path.read_bytes())
    return files


def describe(files: dict[str, str], commit: str) -> dict:
    cargo = (DEST / "engine" / "Cargo.toml").read_text()
    version = re.search(r'^version\s*=\s*"([^"]+)"', cargo, re.MULTILINE)
    rules = json.loads((DEST / "engine" / "institution.json").read_text())
    manifest = json.loads(PDFIUM_MANIFEST.read_text()) if PDFIUM_MANIFEST.exists() else {}
    return {
        "upstream": UPSTREAM,
        "source_commit": commit,
        "engine_version": version.group(1) if version else None,
        "rules_version": rules["version"],
        "adapter_contract": ADAPTER_CONTRACT,
        "pdfium_manifest_version": manifest.get("version"),
        "engine_source_sha256": tree_digest(files, "engine/"),
        "fixtures_sha256": tree_digest(files, "tests/"),
        "files": files,
    }


def vendor(source: Path, commit: str) -> None:
    resolved = subprocess.check_output(["git", "-C", str(source), "rev-parse", f"{commit}^{{commit}}"], text=True).strip()
    archive = subprocess.check_output(["git", "-C", str(source), "archive", "--format=tar", resolved, *PATHS])
    if DEST.exists():
        shutil.rmtree(DEST)
    DEST.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for member in tar.getmembers():
            name = Path(member.name)
            if name.is_absolute() or ".." in name.parts:
                raise SystemExit(f"unsafe path in archive: {member.name}")
            if any(member.name == skip or member.name.startswith(skip + "/") for skip in SKIP):
                continue
            if member.isdir():
                continue
            if not member.isfile():
                raise SystemExit(f"refusing non-regular archive member: {member.name}")
            if member.name.endswith(".pdf") and not member.name.startswith("tests/fixtures/public/"):
                raise SystemExit(f"refusing non-public PDF: {member.name}")
            target = DEST / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(tar.extractfile(member).read())
    PROVENANCE.write_text(json.dumps(describe(snapshot_files(), resolved), indent=2, ensure_ascii=False) + "\n")
    print(f"vendored {UPSTREAM}@{resolved[:12]} into {DEST.relative_to(ROOT)}")


def check() -> int:
    recorded = json.loads(PROVENANCE.read_text())
    actual = describe(snapshot_files(), recorded["source_commit"])
    if actual != recorded:
        changed = sorted(set(actual["files"].items()) ^ set(recorded["files"].items()))
        for name, _ in changed[:20]:
            print(f"drift: {name}", file=sys.stderr)
        for key in actual:
            if key != "files" and actual[key] != recorded.get(key):
                print(f"drift: {key}", file=sys.stderr)
        return 1
    print(f"vendor/inspcal matches {UPSTREAM}@{recorded['source_commit'][:12]}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--source", type=Path, help="InspCal checkout")
    parser.add_argument("--commit", default="HEAD")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.check:
        return check()
    if not args.source:
        parser.error("--source is required unless --check")
    vendor(args.source.expanduser().resolve(), args.commit)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
