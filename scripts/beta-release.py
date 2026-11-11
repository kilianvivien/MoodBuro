"""Validate beta versions and collect a universal Tauri update for GitHub Releases."""
import argparse
import hashlib
import json
import re
import shutil
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPO = "kilianvivien/MoodBuro"


def validate(tag):
    if not re.fullmatch(r"v0\.[1-9][0-9]*\.[0-9]+-beta\.[1-9][0-9]*", tag):
        raise ValueError("Expected v0.MINOR.PATCH-beta.N (beta must be newer than 0.1.0)")
    version = tag[1:]
    base = tuple(map(int, version.split("-")[0].split(".")))
    if base <= (0, 1, 0):
        raise ValueError("Beta versions must be newer than 0.1.0")
    import tomllib
    versions = [json.loads((ROOT / p).read_text())["version"] for p in
                ["package.json", "apps/shell/package.json", "src-tauri/tauri.conf.json"]]
    versions.append(tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"])
    if any(v != version for v in versions):
        raise ValueError("Tag, workspace, app and package versions must all match")
    return version


def forward_only(tag, previous):
    def order(version):
        match = re.fullmatch(r"v?(0)\.([0-9]+)\.([0-9]+)-beta\.([0-9]+)", version)
        if not match:
            raise ValueError("Invalid beta feed version")
        return tuple(map(int, match.groups()))
    if order(tag) <= order(json.loads(previous.read_text())["version"]):
        raise ValueError("Beta feed may only advance to a newer version")


def collect(tag, bundle, output, notes=None):
    version = validate(tag)
    output.mkdir(parents=True, exist_ok=True)
    archives = list((bundle / "macos").glob("*.app.tar.gz"))
    dmgs = list((bundle / "dmg").glob("*.dmg"))
    if len(archives) != 1 or len(dmgs) != 1:
        raise ValueError("Expected one universal app archive and one DMG")
    archive = archives[0]
    sig = archive.with_name(archive.name + ".sig")
    signature = sig.read_text().strip()
    if not signature:
        raise ValueError("Missing update signature")
    for source in [archive, sig, dmgs[0]]:
        shutil.copy2(source, output / source.name)
    platform = {"url": f"https://github.com/{REPO}/releases/download/{tag}/{archive.name}", "signature": signature}
    manifest = {"version": version, "notes": notes.read_text() if notes else f"MoodBuro {version}\nhttps://github.com/{REPO}/releases/tag/{tag}",
                "pub_date": datetime.now(timezone.utc).isoformat(),
                "platforms": {"darwin-aarch64": platform, "darwin-x86_64": platform}}
    (output / "beta.json").write_text(json.dumps(manifest, indent=2) + "\n")
    files = sorted(p for p in output.iterdir() if p.is_file() and p.name != "SHA256SUMS.txt")
    (output / "SHA256SUMS.txt").write_text("".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n" for p in files))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag")
    parser.add_argument("--notes", type=Path)
    parser.add_argument("--previous", type=Path)
    parser.add_argument("--bundle", type=Path)
    parser.add_argument("--output", type=Path, default=Path("artifacts"))
    args = parser.parse_args()
    if args.previous:
        forward_only(args.tag, args.previous)
    if args.bundle:
        collect(args.tag, args.bundle, args.output, args.notes)
    else:
        validate(args.tag)
