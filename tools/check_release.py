"""Verify that a release tag matches the Rust, Python and npm package versions."""

from __future__ import annotations

import argparse
import ast
import json
import os
import sys
import tomllib
from pathlib import Path

DISTRIBUTION = "rocx"


def _manifest_version(root: Path, path: str, table: str) -> str:
    data = tomllib.loads((root / path).read_text(encoding="utf-8"))
    return str(data[table]["version"])


def _lock_version(root: Path, path: str) -> str:
    data = tomllib.loads((root / path).read_text(encoding="utf-8"))
    packages = data.get("package", [])
    if not isinstance(packages, list) or not all(
        isinstance(package, dict) for package in packages
    ):
        raise ValueError(f"{path}: malformed package array")
    versions = [
        str(package["version"])
        for package in packages
        if package.get("name") == DISTRIBUTION
    ]
    if len(versions) != 1:
        raise ValueError(
            f"{path}: expected one {DISTRIBUTION} package, found {len(versions)}"
        )
    return versions[0]


def release_versions(root: Path, tag: str) -> dict[str, str]:
    """Return the tag version and every recorded package version."""
    expected = tag.removeprefix("v")
    if not expected:
        raise ValueError("release tag has no version")
    versions = {
        "tag": expected,
        "Cargo.toml": _manifest_version(root, "Cargo.toml", "package"),
        "pyproject.toml": _manifest_version(root, "pyproject.toml", "project"),
        "Cargo.lock": _lock_version(root, "Cargo.lock"),
        "uv.lock": _lock_version(root, "uv.lock"),
        "javascript/Cargo.lock": _lock_version(root, "javascript/Cargo.lock"),
        "javascript/package.json": json.loads(
            (root / "javascript/package.json").read_text(encoding="utf-8")
        )["version"],
    }
    runtime = "python/rocx/__init__.py"
    versions[runtime] = next(
        (ast.literal_eval(node.value) for node in ast.parse((root / runtime).read_text(encoding="utf-8")).body
         if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "__version__" for t in node.targets)),
        None,
    )
    mismatched = {
        source: version for source, version in versions.items() if version != expected
    }
    if mismatched:
        details = ", ".join(
            f"{source}={version}" for source, version in mismatched.items()
        )
        raise ValueError(f"release tag {tag} does not match {details}")
    return versions


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--tag",
        default=os.environ.get("GITHUB_REF_NAME", ""),
        help="release tag, for example v0.2.0",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="repository root",
    )
    args = parser.parse_args()
    if not args.tag:
        parser.error("--tag or GITHUB_REF_NAME is required")
    try:
        release_versions(args.root, args.tag)
    except (KeyError, OSError, ValueError) as error:
        print(f"release check failed: {error}", file=sys.stderr)
        return 1
    print(f"release {args.tag} matches the Rust, Python and npm versions")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
