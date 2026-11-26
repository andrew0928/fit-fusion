"""A versioned prerelease must not change the default Homebrew installation."""
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[3] / "scripts/sync-fit-fusion.py"
spec = importlib.util.spec_from_file_location("sync_fit_fusion", SCRIPT)
sync = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync)


class HomebrewReleaseChannels(unittest.TestCase):
    def test_opt_in_prerelease_adds_only_its_versioned_formula(self):
        stable = dict(tag_name="v0.2.3", draft=False, prerelease=False, body=None, assets=[])
        preview = dict(stable, tag_name="v0.2.5", prerelease=True, body=sync.VERSIONED_ONLY)
        batch = [stable, dict(preview, tag_name="v0.2.4", body=""), dict(preview, tag_name="v0.2.6", draft=True)]

        def api_response(*args, **kwargs):
            return io.BytesIO(json.dumps(batch).encode())

        def fake_package(version, assets):
            return f"https://example.test/{version}.tar.gz", "a" * 64, sync.TOOLS, ()

        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp) / "Formula"
            folder.mkdir()
            with patch.object(sync, "urlopen", side_effect=api_response), \
                 patch.object(sync, "package", side_effect=fake_package), \
                 patch("sys.argv", [str(SCRIPT), tmp]):
                sync.main()
                default = (folder / "fit-fusion.rb").read_bytes()
                batch.insert(0, preview)
                sync.main()
                sync.main()
            self.assertEqual(sorted(p.name for p in folder.iterdir()),
                             ["fit-fusion.rb", "fit-fusion@0.2.3.rb", "fit-fusion@0.2.5.rb"])
            self.assertEqual((folder / "fit-fusion.rb").read_bytes(), default)
            self.assertIn(b'version "0.2.3"', default)
            formula = (folder / "fit-fusion@0.2.5.rb").read_text()
            self.assertIn('version "0.2.5"', formula)
            self.assertIn("keg_only :versioned_formula", formula)
