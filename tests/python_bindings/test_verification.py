"""Check that mandatory CI validation cannot silently become optional."""

import os
from pathlib import Path
import subprocess
import sys
from xml.etree import ElementTree

import pytest


@pytest.mark.parametrize("required", [False, True])
def test_missing_schemas_skip_only_when_optional(tmp_path: Path, required: bool) -> None:
    """Exercise the real standards entrypoint with an empty schema directory."""
    root = Path(__file__).resolve().parents[2]
    environment = dict(os.environ, ROCX_SCHEMA_DIR=str(tmp_path),
                       ROCX_REQUIRE_SCHEMAS="1" if required else "0")
    report = tmp_path / "standards-report.xml"
    result = subprocess.run(
        [sys.executable, "-m", "pytest", "-q", "--junitxml", str(report),
         "tests/python_bindings/test_standards.py"],
        cwd=root, env=environment, capture_output=True, text=True, check=False,
    )
    output = result.stdout + result.stderr
    assert result.returncode == (1 if required else 0), output
    cases = ElementTree.parse(report).findall(".//testcase")
    assert cases, output
    outcome = "error" if required else "skipped"
    expected = "Required official schemas missing" if required else "Download pinned schemas"
    for case in cases:
        status = case.find(outcome)
        assert status is not None and expected in status.get("message", ""), case.get("name")
