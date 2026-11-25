"""Check the tag/version gate that runs before publishing."""

import importlib.util
import json
from pathlib import Path

import pytest

ROOT = Path(__file__).parents[2]
spec = importlib.util.spec_from_file_location(
    "check_release", ROOT / "tools/check_release.py"
)
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


def write_tree(root: Path, **overrides: str) -> None:
    versions = {
        "cargo": "0.2.0",
        "pyproject": "0.2.0",
        "cargo_lock": "0.2.0",
        "uv_lock": "0.2.0",
        "wasm_lock": "0.2.0",
        "npm": "0.2.0",
        "runtime": "0.2.0",
    }
    versions.update(overrides)
    (root / "Cargo.toml").write_text(
        f'[package]\nname = "rocx"\nversion = "{versions["cargo"]}"\n',
        encoding="utf-8",
    )
    (root / "pyproject.toml").write_text(
        f'[project]\nname = "rocx"\nversion = "{versions["pyproject"]}"\n',
        encoding="utf-8",
    )
    (root / "javascript").mkdir()
    (root / "javascript/package.json").write_text(json.dumps({"name": "rocx-docx", "version": versions["npm"]}))
    (root / "python/rocx").mkdir(parents=True)
    (root / "python/rocx/__init__.py").write_text(f'__version__ = "{versions["runtime"]}"\n')
    for name, key in (("Cargo.lock", "cargo_lock"), ("uv.lock", "uv_lock"), ("javascript/Cargo.lock", "wasm_lock")):
        (root / name).write_text(
            f'[[package]]\nname = "rocx"\nversion = "{versions[key]}"\n',
            encoding="utf-8",
        )


def test_matching_versions_pass(tmp_path: Path) -> None:
    write_tree(tmp_path)
    versions = release.release_versions(tmp_path, "v0.2.0")
    assert versions["tag"] == "0.2.0"
    assert set(versions.values()) == {"0.2.0"}


@pytest.mark.parametrize(
    ("source", "keyword"),
    [
        ("Cargo.toml", "cargo"),
        ("pyproject.toml", "pyproject"),
        ("Cargo.lock", "cargo_lock"),
        ("uv.lock", "uv_lock"),
        ("javascript/Cargo.lock", "wasm_lock"),
        ("javascript/package.json", "npm"),
        ("python/rocx/__init__.py", "runtime"),
    ],
)
def test_mismatched_version_fails(tmp_path: Path, source: str, keyword: str) -> None:
    write_tree(tmp_path, **{keyword: "0.2.1"})
    with pytest.raises(ValueError, match=source):
        release.release_versions(tmp_path, "v0.2.0")


@pytest.mark.parametrize("name", ["Cargo.lock", "uv.lock"])
def test_missing_lock_entry_fails(tmp_path: Path, name: str) -> None:
    write_tree(tmp_path)
    (tmp_path / name).write_text(
        '[[package]]\nname = "other"\nversion = "0.2.0"\n', encoding="utf-8"
    )
    with pytest.raises(ValueError, match="expected one"):
        release.release_versions(tmp_path, "v0.2.0")


@pytest.mark.parametrize("name", ["Cargo.lock", "uv.lock"])
def test_malformed_lock_fails(tmp_path: Path, name: str) -> None:
    write_tree(tmp_path)
    (tmp_path / name).write_text('[package]\nname = "rocx"\n', encoding="utf-8")
    with pytest.raises(ValueError, match="malformed package array"):
        release.release_versions(tmp_path, "v0.2.0")
