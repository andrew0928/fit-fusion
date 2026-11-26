"""M5-5 acceptance with isolated HOME, synthetic FITs and actual terminal output."""
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import pty
import re
import subprocess
import tempfile
import unittest

SRC = Path(__file__).resolve().parents[2]
FIXTURES = SRC / "tools/tests/fixtures/activities"


class PersonalSettings(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binaries = {}
        for crate in ("query-files", "query-apple", "query-garmin", "pair-merge", "inspector"):
            name = "fit-merge" if crate == "pair-merge" else "fit-" + crate
            output = subprocess.check_output([
                "cargo", "build", "--locked", "--manifest-path", SRC / "rust" / crate / "Cargo.toml",
                "--bin", name, "--message-format=json"
            ], text=True)
            cls.binaries[name] = next(r["executable"] for r in map(json.loads, output.splitlines())
                                      if r.get("executable") and r["target"]["name"] == name)

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.env = dict(os.environ, HOME=tmp.name, TERM="xterm-256color")
        self.env.pop("NO_COLOR", None)
        self.config = self.root / ".fit-fusion/config.yaml"

    def run_cli(self, tool, *args, code=0):
        result = subprocess.run([self.binaries[tool], *args], cwd=self.root, env=self.env,
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, code, result.stderr)
        return result

    def test_author_topic_without_working_config(self):
        self.config.parent.mkdir()
        self.config.write_text("invalid: [")
        author = (SRC / "python/fit_fusion/guides/author.md").read_text()
        for tool in self.binaries:
            with self.subTest(tool=tool):
                self.assertIn(f"{tool} --skill author", self.run_cli(tool, "--skill", "index").stdout)
                result = self.run_cli(tool, "--skill", "author")
                self.assertEqual(result.stdout, author)
                self.assertEqual(result.stderr, "")

    def test_init_is_atomic_idempotent_and_preserves_user_file(self):
        tools = ["fit-query-files", "fit-query-apple", "fit-query-garmin"] * 2
        def init(tool):
            return json.loads(self.run_cli(tool, "config", "init").stdout)["status"]
        with ThreadPoolExecutor(max_workers=len(tools)) as pool:
            self.assertCountEqual(pool.map(init, tools), ["created"] + ["already_exists"] * 5)
        self.assertEqual(self.config.stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.config.parent.stat().st_mode & 0o777, 0o700)
        self.assertTrue(all(not s.strip() or s.startswith("#") for s in self.config.read_text().splitlines()))
        self.run_cli(tools[0], "--dir", ".", "list")
        self.config.write_text("# personal\n")
        stamp = self.config.stat().st_mtime_ns
        self.assertEqual(init(tools[1]), "already_exists")
        self.assertEqual((self.config.read_text(), self.config.stat().st_mtime_ns), ("# personal\n", stamp))
        self.config.unlink()
        target = self.root / "untouched"
        self.config.symlink_to(target)
        self.run_cli(tools[2], "config", "init", code=2)
        self.assertFalse(target.exists())

    def configure_files(self):
        self.config.parent.mkdir()
        (self.config.parent / "騎乘").mkdir()
        self.config.write_text("version: 1\ncommon: {timezone: UTC}\ntools:\n  fit-query-files: {dir: 騎乘}\n")

    def test_sources_and_machine_output(self):
        self.configure_files()
        run = self.run_cli("fit-query-files", "list", "--date", "2026-11-21", "--json")
        self.assertEqual(json.loads(run.stdout)["activities"], [])
        params = json.loads(run.stderr)["parameters"]
        self.assertEqual(params["dir"], {"source":"config", "detail":"tools.fit-query-files.dir",
                                         "value":str(self.config.parent / "騎乘")})
        self.assertEqual(params["date"]["source"], "cli")
        self.assertEqual(params["timezone"]["source"], "config")
        self.assertEqual(params["timezone"]["value"], "UTC")
        run = self.run_cli("fit-query-files", "--timezone", "Asia/Taipei", "list", "--json")
        self.assertEqual(json.loads(run.stderr)["parameters"]["timezone"]["detail"], "--timezone")

    def test_terminal_colors_and_pipes(self):
        self.configure_files()
        args = [self.binaries["fit-query-files"], "list"]
        plain = self.run_cli("fit-query-files", "list").stderr
        self.assertNotIn("\x1b", plain)
        self.assertTrue(all(row.startswith("- ") for row in plain.splitlines()))
        for extra, color in (({}, True), ({"NO_COLOR": "1"}, False), ({"TERM": "dumb"}, False)):
            master, slave = pty.openpty()
            try:
                result = subprocess.run(args, env={**self.env, **extra}, cwd=self.root,
                                        stdout=subprocess.PIPE, stderr=slave, check=True)
                self.assertEqual(json.loads(result.stdout)["status"], "ok")
                terminal = os.read(master, 8192).decode().replace("\r\n", "\n")
            finally:
                os.close(master)
                os.close(slave)
            self.assertEqual(re.sub(r"\x1b\[\d+m", "", terminal), plain)
            for sequence in ("\x1b[31mdir", "\x1b[37m", "\x1b[90mconfig"):
                self.assertEqual(sequence in terminal, color)

    def test_errors_and_offline_help(self):
        self.config.parent.mkdir()
        self.config.with_suffix(".json").write_text('{"version": 123}')
        self.run_cli("fit-query-files", "--dir", ".", "list")
        for invalid in ("{}", "null", "version: 2", "tools: {}", "version: 1\ntools: [", "version: 1\ncommon: []"):
            self.config.write_text(invalid)
            result = self.run_cli("fit-query-files", "--dir", ".", "list", code=2)
            self.assertIn(str(self.config), result.stderr)
        self.config.write_text("version: 1\ncommon: {timezone: Unknown/Zone}")
        self.assertIn(str(self.config), self.run_cli("fit-query-files", "list", code=2).stderr)
        self.run_cli("fit-query-files", "--dir", ".", "--timezone", "UTC", "list")
        for tool in self.binaries:
            for flag in ("--help", "--skill", "--version"):
                self.assertFalse(self.run_cli(tool, flag).stderr)
        self.config.write_text("version: 1\ntools: {fit-query-garmin: {env_file: absent.env, cache_dir: cache}}")
        result = self.run_cli("fit-query-garmin", "read", "--id", "42", "--offline", "--json", code=3)
        self.assertEqual(json.loads(result.stdout)["error"]["code"], "cache_miss")
        self.assertNotIn("env-file", json.loads(result.stderr)["parameters"])

    def test_apple_and_garmin_resolve_paths_without_logging_secrets(self):
        self.config.parent.mkdir()
        self.config.write_text(
            'version: 1\ntools:\n  fit-query-apple: {archive: "~/unmounted.zip"}\n'
            '  fit-query-garmin: {env_file: garmin.env}\n'
        )
        result = self.run_cli("fit-query-apple", "list", "--json", code=3)
        archive = json.loads(result.stderr)["parameters"]["archive"]
        self.assertEqual((archive["value"], archive["source"]), (str(self.root / "unmounted.zip"), "config"))
        secret = "example-invalid-key-with-a-space"
        (self.config.parent / "garmin.env").write_text("GARMIN_PERSONAL_ACCESS_TOKEN=" + secret + " x")
        result = self.run_cli("fit-query-garmin", "list", "--json", code=4)
        entry = json.loads(result.stderr)["parameters"]["env-file"]
        self.assertEqual((entry["value"], entry["source"]), (str(self.config.parent / "garmin.env"), "config"))
        self.assertNotIn(secret, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["error"]["code"], "garmin_auth")

    def test_export_merge_inspect_with_personal_settings(self):
        self.config.parent.mkdir()
        self.config.write_text(f"version: 1\ntools:\n  fit-query-files:\n    dir: {FIXTURES}\n")
        files = json.loads(self.run_cli("fit-query-files", "list", "--json").stdout)["activities"]
        exported = {}
        for activity in files:
            name = Path(activity["path"]).name
            if name in ("rider.fit", "cirqa.fit"):
                run = self.run_cli("fit-query-files", "export", "--id", activity["id"], "--out", "inputs", "--json")
                exported[name] = json.loads(run.stdout)["exports"][0]["path"]
        result = self.run_cli("fit-merge", "--primary", exported["rider.fit"], "--overlay", exported["cirqa.fit"],
                              "--stream", "record.heart_rate", "--out", "merged.fit", "--json")
        self.assertEqual(json.loads(result.stderr)["parameters"]["stream"]["source"], "cli")
        inspect = self.run_cli("fit-inspector", "inspect", "merged.fit", "--stream", "record.heart_rate")
        self.assertIn("record.heart_rate", json.dumps(json.loads(inspect.stdout)))
        self.assertRegex(inspect.stderr, r"(?m)^- decoder +rustyfit +cli \(default\)$")
