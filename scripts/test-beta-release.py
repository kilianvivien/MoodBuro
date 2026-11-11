"""Release contract tests; no network or signing secrets required."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("release", Path(__file__).with_name("beta-release.py"))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)

class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for name in ["package.json", "apps/shell/package.json", "src-tauri/tauri.conf.json"]:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps({"version": "0.2.0-beta.2"}))
        (self.root / "Cargo.toml").write_text('[workspace.package]\nversion="0.2.0-beta.2"\n')
        self.patch = patch.object(release, "ROOT", self.root)
        self.patch.start()
        self.addCleanup(self.patch.stop)

    def test_versions_and_channel(self):
        self.assertEqual(release.validate("v0.2.0-beta.2"), "0.2.0-beta.2")
        for tag in ["v0.2.0", "v0.2.0-alpha.1", "v0.2.0-beta.1", "v0.0.0-beta.1", "v0.1.0-beta.1"]:
            with self.assertRaises(ValueError):
                release.validate(tag)

    def test_feed_only_advances(self):
        previous = self.root / "previous.json"
        previous.write_text('{"version":"0.2.0-beta.2"}')
        release.forward_only("v0.2.0-beta.10", previous)
        for tag in ["v0.2.0-beta.1", "v0.2.0-beta.2", "v0.1.9-beta.3"]:
            with self.assertRaises(ValueError):
                release.forward_only(tag, previous)

    def test_manifest_uses_signed_archive_for_both_architectures(self):
        bundle = self.root / "bundle"
        (bundle / "macos").mkdir(parents=True)
        (bundle / "dmg").mkdir()
        (bundle / "macos/MoodBuro.app.tar.gz").write_bytes(b"fixture archive")
        (bundle / "dmg/MoodBuro.dmg").write_bytes(b"fixture dmg")
        sig = bundle / "macos/MoodBuro.app.tar.gz.sig"
        sig.write_text("signature-content\n")
        output = self.root / "output"
        release.collect("v0.2.0-beta.2", bundle, output)
        manifest = json.loads((output / "beta.json").read_text())
        platforms = manifest["platforms"]
        self.assertEqual(set(platforms), {"darwin-aarch64", "darwin-x86_64"})
        self.assertEqual(platforms["darwin-aarch64"]["signature"], "signature-content")
        self.assertIn("/v0.2.0-beta.2/MoodBuro.app.tar.gz", platforms["darwin-x86_64"]["url"])
        self.assertEqual(len((output / "SHA256SUMS.txt").read_text().splitlines()), 4)
        sig.write_text("")
        with self.assertRaises(ValueError):
            release.collect("v0.2.0-beta.2", bundle, output)

if __name__ == "__main__":
    unittest.main()
