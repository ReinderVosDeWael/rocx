"""Check that mandatory CI validation cannot silently become optional."""

import os
from pathlib import Path
import subprocess
import sys

import pytest


@pytest.mark.parametrize("required", [False, True])
def test_missing_schemas_skip_only_when_optional(tmp_path: Path, required: bool) -> None:
    """Exercise the real standards entrypoint with an empty schema directory."""
    root = Path(__file__).resolve().parents[2]
    environment = dict(os.environ, ROCX_SCHEMA_DIR=str(tmp_path),
                       ROCX_REQUIRE_SCHEMAS="1" if required else "0")
    result = subprocess.run(
        [sys.executable, "-m", "pytest", "-q", "tests/python_bindings/test_standards.py"],
        cwd=root, env=environment, capture_output=True, text=True, check=False,
    )
    output = result.stdout + result.stderr
    if required:
        assert result.returncode == 1, output
        assert "Required official schemas missing" in output
        assert "29 errors" in output
    else:
        assert result.returncode == 0, output
        assert "29 skipped" in output
