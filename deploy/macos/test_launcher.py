import importlib.util
import os
from pathlib import Path
import plistlib
import stat
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("hypermind.py")
SPEC = importlib.util.spec_from_file_location("hypermind_macos_launcher", SCRIPT)
launcher = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(launcher)


class LauncherTests(unittest.TestCase):
    def test_plist_round_trip_keeps_special_paths_and_argv(self):
        state = "/Users/example/Library/Application Support/Memory & <facts> 'quoted'"
        binary = '/Users/example/My Project "quoted"/hm'
        args = launcher.arguments(["render", "--state", state, "--binary", binary])
        encoded = launcher.plist_bytes(args)
        plist = plistlib.loads(encoded)
        self.assertEqual(plist["ProgramArguments"], [sys.executable, str(SCRIPT.resolve()), "supervise", "--state", state, "--binary", binary])
        self.assertEqual(plist["Umask"], 0o077)
        self.assertEqual(plist["WorkingDirectory"], state)
        self.assertTrue(plist["KeepAlive"])
        self.assertIn(b"&amp;", encoded)
        self.assertIn(b"&lt;facts&gt;", encoded)
        rendered = subprocess.run([sys.executable, str(SCRIPT), "render", "--state", state, "--binary", binary], check=True, capture_output=True).stdout
        self.assertEqual(plistlib.loads(rendered), plist)

    def test_custom_plist_is_preserved_without_explicit_replace(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "custom.plist"
            original = plistlib.dumps({"Label": "custom", "ProgramArguments": ["custom-command"]})
            path.write_bytes(original)
            path.chmod(0o600)
            with self.assertRaisesRegex(RuntimeError, "preserved"):
                launcher.install_plist(path, b"new")
            self.assertEqual(path.read_bytes(), original)
            replacement = launcher.plist_bytes(launcher.arguments(["render"]))
            launcher.install_plist(path, replacement, replace=True)
            self.assertEqual(path.read_bytes(), replacement)
            backups = list(path.parent.glob("*.backup"))
            self.assertEqual(len(backups), 1)
            self.assertEqual(backups[0].read_bytes(), original)
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            launcher.install_plist(path, replacement)
            self.assertEqual(len(list(path.parent.glob("*.backup"))), 1)

    def test_private_files_and_symlinks(self):
        with tempfile.TemporaryDirectory() as directory:
            state = Path(directory) / "state with spaces"
            launcher.private_directory(state)
            self.assertEqual(stat.S_IMODE(state.stat().st_mode), 0o700)
            secret = state / "hypermind.conf"
            launcher.private_file(secret, create=True)
            secret.chmod(0o644)
            with self.assertRaisesRegex(RuntimeError, "private"):
                launcher.private_file(secret)
            link = Path(directory) / "link"
            link.symlink_to(state, target_is_directory=True)
            with self.assertRaisesRegex(RuntimeError, "symlink"):
                launcher.private_directory(link)

    def test_real_lock_prevents_two_managed_writers(self):
        with tempfile.TemporaryDirectory() as directory:
            state = Path(directory) / "state"
            with launcher.writer_lock(state):
                with self.assertRaisesRegex(RuntimeError, "another managed process"):
                    with launcher.writer_lock(state):
                        self.fail("second writer admitted")
            with launcher.writer_lock(state):
                pass

    def test_all_docker_plans_preserve_volume_and_single_writer(self):
        for action in ("start", "stop", "status", "logs", "doctor", "build"):
            args = launcher.arguments(["docker", "--action", action, "--render-plan"])
            plan = launcher.docker_plan(args)
            self.assertIn("--project-name", plan["prefix"])
            self.assertEqual(plan["project"], "hypermind")
            for step in plan["steps"]:
                self.assertFalse(set(step["arguments"]) & {"down", "--volumes", "-v"})
            if action == "start":
                self.assertEqual(plan["steps"][0]["condition"], "daemon_not_running")
                self.assertIn("--if-missing", plan["steps"][0]["arguments"])
                self.assertIn("hypermind=1", plan["steps"][1]["arguments"])

    @unittest.skipIf(sys.platform == "darwin", "Linux guard only")
    def test_linux_does_not_attempt_native_installation(self):
        result = subprocess.run([sys.executable, str(SCRIPT), "install"], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("native lifecycle requires macOS", result.stderr)


if __name__ == "__main__":
    unittest.main()
