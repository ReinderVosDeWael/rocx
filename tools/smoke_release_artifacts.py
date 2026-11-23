"""Run the published quickstarts against a built wheel or packaged crate."""

import argparse
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import tomllib
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DOC = ROOT / "docs/website/getting-started.md"
W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def quickstart(markdown: str, language: str) -> str:
    """Extract exactly one language snippet from the install section."""
    section = markdown.split("## Run the repository examples", 1)[0]
    matches = re.findall(
        rf"^    ```{language}\s*\n(.*?)^    ```\s*$",
        section, re.MULTILINE | re.DOTALL,
    )
    if len(matches) != 1:
        raise ValueError(f"expected exactly one {language} quickstart, found {len(matches)}")
    return textwrap.dedent(matches[0])


def verify_document(path: Path) -> None:
    with zipfile.ZipFile(path) as package:
        root = ET.fromstring(package.read("word/document.xml"))
    paragraphs = ["".join(t.text or "" for t in p.iter(W + "t"))
                  for p in root.iter(W + "p")]
    if "Hello, world!" not in paragraphs:
        raise ValueError(f"expected quickstart paragraph, got {paragraphs!r}")


def run(command: list[str], cwd: Path) -> None:
    subprocess.run(command, cwd=cwd, check=True)


def wheel_smoke(wheel: Path, directory: Path, version: str, source: str) -> None:
    if not wheel.is_file() or wheel.suffix != ".whl":
        raise ValueError(f"not a wheel: {wheel}")
    with zipfile.ZipFile(wheel) as archive:
        if any(name.endswith(".pth") for name in archive.namelist()):
            raise ValueError("editable/path-injecting wheel is not a release artifact")
    env = directory / "env"
    run(["uv", "venv", "--python", sys.executable, str(env)], directory)
    python = env / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
    run(["uv", "pip", "install", "--python", str(python), "--no-deps", str(wheel.resolve())], directory)
    probe = directory / "probe.py"
    probe.write_text(
        "import importlib.metadata, importlib.machinery, pathlib, sys\n"
        "import rocx, rocx._native\n"
        f"assert rocx.__version__ == {version!r}\n"
        f"assert importlib.metadata.version('rocx') == {version!r}\n"
        "assert rocx._native.BACKEND == 'rust'\n"
        "assert pathlib.Path(rocx.__file__).resolve().is_relative_to(pathlib.Path(sys.prefix).resolve())\n"
        "assert pathlib.Path(rocx._native.__file__).resolve().is_relative_to(pathlib.Path(sys.prefix).resolve())\n"
        "assert any(rocx._native.__file__.endswith(s) for s in importlib.machinery.EXTENSION_SUFFIXES)\n",
        encoding="utf-8",
    )
    report = directory / "report.py"
    report.write_text(source, encoding="utf-8")
    run([str(python), "-I", str(probe)], directory)
    run([str(python), "-I", str(report)], directory)
    verify_document(directory / "report.docx")


def crate_smoke(crate: Path, directory: Path, version: str, source: str) -> None:
    # Cargo-produced archives contain one top-level package directory.
    with tarfile.open(crate, "r:gz") as archive:
        archive.extractall(directory, filter="data")
    package = directory / f"rocx-{version}"
    if not (package / "Cargo.toml").is_file():
        raise ValueError("archive does not contain the expected package")
    consumer = directory / "consumer"
    (consumer / "src").mkdir(parents=True)
    # TOML basic strings accept JSON escaping, including Windows paths.
    import json
    (consumer / "Cargo.toml").write_text(
        '[package]\nname = "quickstart-consumer"\nversion = "0.1.0"\nedition = "2024"\n'
        f'[dependencies]\nrocx = {{ path = {json.dumps(str(package))}, version = "={version}" }}\n'
        'futures-executor = "0.3"\n', encoding="utf-8")
    (consumer / "src/main.rs").write_text(source, encoding="utf-8")
    run(["cargo", "run"], consumer)
    verify_document(consumer / "report.docx")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--wheel", type=Path)
    group.add_argument("--crate", type=Path)
    args = parser.parse_args()
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    markdown = DOC.read_text(encoding="utf-8")
    print(f"Testing on {platform.system()} {platform.machine()} (Python {platform.python_version()})", flush=True)
    with tempfile.TemporaryDirectory(prefix="rocx-artifact-") as temp:
        directory = Path(temp)
        if args.wheel:
            wheel_smoke(args.wheel, directory, version, quickstart(markdown, "python"))
        else:
            crate_smoke(args.crate.resolve(), directory, version, quickstart(markdown, "rust"))
    print("Packaged quickstart passed", flush=True)


if __name__ == "__main__":
    main()
