"""Exercise the packed runner's scratch lifecycle without building an artifact."""

import os
from pathlib import Path
import select
import signal
import subprocess
import tempfile
import unittest


RUNNER = Path(__file__).with_name("check-packed.sh").resolve()


class PackedScratchTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="rocx-packed-test-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.scratch = self.root / "scratch with spaces"
        self.scratch.mkdir()
        self.sentinel = self.scratch / "rocx-packed-js.other-run"
        self.sentinel.mkdir()
        self.env = os.environ | {
            "PATH": f"{self.root}:{os.environ['PATH']}",
            "TMPDIR": self.scratch.name,
        }
        self.stub("node", "echo v22.0.0")

    def stub(self, name, script):
        path = self.root / name
        path.write_text(f"#!/bin/bash\n{script}\n")
        path.chmod(0o700)

    def start(self, *args):
        runner = subprocess.Popen(
            ["bash", RUNNER, *args],
            cwd=self.root,
            env=self.env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            start_new_session=True,
        )
        self.addCleanup(self.stop, runner)
        return runner

    @staticmethod
    def stop(runner):
        if runner.poll() is None:
            os.killpg(runner.pid, signal.SIGKILL)
        runner.communicate(timeout=10)

    def outputs(self):
        self.assertTrue(self.sentinel.is_dir())
        return sorted(set(self.scratch.iterdir()) - {self.sentinel})

    def test_failure_removes_only_its_scratch(self):
        self.stub("uv", "exit 42")
        runner = self.start()
        runner.communicate(timeout=10)
        self.assertEqual(runner.returncode, 42)
        self.assertEqual(self.outputs(), [])

    def test_keep_output_retains_failed_run(self):
        self.stub("uv", "exit 42")
        runner = self.start("--keep-output")
        stdout, _ = runner.communicate(timeout=10)
        self.assertEqual(runner.returncode, 42)
        [output] = self.outputs()
        self.assertIn(str(output), stdout)
        self.assertTrue(output.is_dir())

    def test_unknown_option_creates_no_scratch(self):
        runner = self.start("--typo")
        _, stderr = runner.communicate(timeout=10)
        self.assertEqual(runner.returncode, 2)
        self.assertIn("--keep-output", stderr)
        self.assertEqual(self.outputs(), [])

    def test_interruption_cleans_scratch_and_preserves_status(self):
        self.stub("uv", "echo ready >&2\nexec sleep 30")
        for sig in (signal.SIGHUP, signal.SIGINT, signal.SIGTERM):
            with self.subTest(signal=sig):
                runner = self.start()
                self.assertTrue(select.select([runner.stderr], [], [], 10)[0])
                self.assertEqual(runner.stderr.readline().strip(), "ready")
                self.assertEqual(len(self.outputs()), 1)
                os.killpg(runner.pid, sig)
                runner.communicate(timeout=10)
                self.assertEqual(runner.returncode, 128 + sig)
                self.assertEqual(self.outputs(), [])


if __name__ == "__main__":
    unittest.main()
