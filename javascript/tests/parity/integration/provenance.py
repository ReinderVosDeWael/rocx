"""Freeze, build, replay and audit the installed rocx-docx release bytes."""

import argparse
import base64
import hashlib
from importlib import metadata
import io
import json
import os
from pathlib import Path, PurePosixPath
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile


# Byte-for-byte manifest, not a Git index: untracked files under these inputs
# are included. Cargo's clean replay also rejects any missing include/build input.
SOURCE = (
    "LICENSE", "README.md", "Cargo.toml", "Cargo.lock", "src", "assets",
    "examples/javascript", "tests/fixtures/images/sample.png",
    "javascript/src", "javascript/types",
)
EXTERNAL_CONFIG = (".cargo", ".npmrc", "rust-toolchain", "rust-toolchain.toml")
PACK_INPUTS = (".gitignore", ".npmignore", "javascript/.gitignore", "javascript/.npmignore")
TEST_INPUTS = (
    "javascript/tests", "examples", "tests/fixtures", "tests/support", "tests/javascript_parity",
    "tests/authoring_builders.rs", "tests/run_format_builders.rs", "tools", "python/rocx",
    "pyproject.toml", "uv.lock", ".python-version", ".github/workflows/javascript.yml",
)
PRODUCTS = ("pkg/rocx_bg.wasm", "pkg/rocx.js", "target/wasm32-unknown-unknown/release/rocx_js.wasm")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    require(not path.is_symlink() and path.is_file(), f"expected a regular file: {path}")
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def npm_integrity(path):
    with path.open("rb") as file:
        return "sha512-" + base64.b64encode(hashlib.file_digest(file, "sha512").digest()).decode("ascii")


def files(root, entries, *, ignore_python_cache=False):
    manifest = {}

    def visit(path):
        name = path.relative_to(root).as_posix()
        if ignore_python_cache and (path.name == "__pycache__" or path.suffix == ".pyc"):
            return
        require(not path.is_symlink(), f"symlink is not a frozen input: {path}")
        if path.is_dir():
            for child in sorted(path.iterdir()):
                visit(child)
        else:
            manifest[name] = file_digest(path)

    for entry in entries:
        visit(root / entry)
    return manifest


def equal(expected, actual, label):
    changes = sorted(name for name in expected.keys() | actual.keys() if expected.get(name) != actual.get(name))
    require(not changes, f"{label}: differing/missing/unexpected files: {', '.join(changes)}")


def inventory(root):
    # All top-level adapter/build files, excluding only generated products and
    # ignore files (recorded in the separate packaging input list).
    adapter = sorted(
        path.relative_to(root).as_posix() for path in (root / "javascript").iterdir()
        if (path.is_file() or path.is_symlink()) and path.name not in {
            "LICENSE", "rocx-source.tar.gz", ".gitignore", ".npmignore",
        }
    )
    sources = (*SOURCE, *adapter)
    for directory in (root, root / "javascript"):
        for name in (*EXTERNAL_CONFIG, "build.rs"):
            path = directory / name
            if path.exists() or path.is_symlink():
                relative = path.relative_to(root).as_posix()
                if relative not in sources:
                    sources += (relative,)
    ignores = [name for name in PACK_INPUTS if (root / name).exists() or (root / name).is_symlink()]
    return {
        "source": files(root, sources),
        "packaging": files(root, ignores),
        "tests": files(root, TEST_INPUTS, ignore_python_cache=True),
    }


def command(*args, cwd=None, env=None):
    result = subprocess.run(args, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE, check=True)
    return result.stdout.strip()


def tools(cwd):
    return {
        "cargo": command("cargo", "-Vv", cwd=cwd),
        "rustc": command("rustc", "-Vv", cwd=cwd),
        "wasm_bindgen": command("wasm-bindgen", "--version", cwd=cwd),
        "node": command("node", "--version", cwd=cwd),
        "npm": command("npm", "--version", cwd=cwd),
        "uv": command("uv", "--version", cwd=cwd),
        "bash": command("bash", "--version", cwd=cwd).splitlines()[0],
        "tar": command("tar", "--version", cwd=cwd).splitlines()[0],
        "python": sys.version,
        "platform": platform.platform(),
    }


def build_environment(tree):
    for name in os.environ:
        require(
            not name.startswith(("CARGO_BUILD_", "CARGO_TARGET_", "CARGO_PROFILE_", "RUSTC"))
            and name not in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_INCREMENTAL"},
            f"unset {name}: provenance requires an unoverridden Rust release build",
        )
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).resolve()
    # Cargo can otherwise load side inputs outside the shipped source, including
    # compiler wrappers or alternate registries. Do not capture secret config.
    for directory in {cargo_home, *(p / ".cargo" for p in tree.parents)}:
        for name in ("config", "config.toml"):
            path = directory / name
            require(not path.exists() and not path.is_symlink(),
                    f"external Cargo config {path}; use an isolated scratch/Cargo home without it")
    return os.environ | {"CARGO_HOME": str(cargo_home)}


def crate_roots(tree, env):
    metadata = json.loads(command(
        "cargo", "metadata", "--locked", "--format-version=1", "--filter-platform=wasm32-unknown-unknown",
        "--manifest-path", "Cargo.toml", cwd=tree / "javascript", env=env,
    ))
    resolved = {node["id"] for node in metadata["resolve"]["nodes"]}
    roots = {}
    for package in metadata["packages"]:
        if package["id"] not in resolved:
            continue
        location = Path(package["manifest_path"]).parent.resolve()
        if package["source"] is None:
            require(location.is_relative_to(tree), f"unshipped Cargo path dependency: {location}")
        else:
            require(package["source"].startswith("registry+"), f"dependency is not registry-locked: {package['id']}")
            roots[package["id"]] = location
    return roots


def crate_files(roots):
    return {key: files(path, ["."]) for key, path in sorted(roots.items())}


def schema_inputs():
    directory = Path(os.environ.get("ROCX_SCHEMA_DIR", Path(tempfile.gettempdir()) / "rocx-validation-schemas"))
    directory = directory.resolve(strict=True)
    schemas = sorted(path.relative_to(directory) for path in directory.rglob("*.xsd"))
    require(schemas, f"no schemas in {directory}; set ROCX_SCHEMA_DIR")
    return {"directory": str(directory), "files": files(directory, schemas)}


def unpack(stream, archive_name, prefix=()):
    """Read regular archive members without permitting archive-controlled writes."""
    entries = {}
    seen = set()
    with tarfile.open(fileobj=stream, mode="r:gz") as archive:
        for item in archive:
            parts = PurePosixPath(item.name).parts
            require(parts and "/" not in parts and ".." not in parts
                    and PurePosixPath(item.name).as_posix() == item.name
                    and parts[:len(prefix)] == prefix,
                    f"{archive_name}: unsafe member {item.name!r}")
            require(item.name not in seen, f"{archive_name}: duplicate member {item.name}")
            seen.add(item.name)
            if item.isdir():
                continue
            require(item.isfile() and len(parts) > len(prefix),
                    f"{archive_name}: not a regular file: {item.name}")
            path = "/".join(parts[len(prefix):])
            with archive.extractfile(item) as file:
                entries[path] = file.read()
    return entries


def archive_contents(artifact, source):
    with artifact.open("rb") as stream:
        package = unpack(stream, artifact.name, ("package",))
    corresponding = unpack(io.BytesIO(package["rocx-source.tar.gz"]), "rocx-source.tar.gz")
    equal(source, {name: digest(data) for name, data in corresponding.items()}, "corresponding source archive")
    for name, data in package.items():
        if name in (*PRODUCTS, "rocx-source.tar.gz"):
            continue
        key = name if name == "LICENSE" else f"javascript/{name}"
        require(source.get(key) == digest(data), f"shipped package file does not match frozen source: {name}")
    return package, corresponding


def package_products(tree, package):
    products = files(tree / "javascript", PRODUCTS)
    for path in PRODUCTS[:2]:
        require(products[path] == digest(package[path]), f"{tree.name}: {path} differs from npm tarball")
    return products


def materialize(root, contents):
    root.mkdir()
    for name, data in contents.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def write_report(output, report):
    path = output / "provenance.json"
    pending = path.with_suffix(".json.pending")
    pending.write_text(json.dumps(report, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    pending.replace(path)


def pack(root, output):
    require(shutil.disk_usage(output).free >= 2 * 1024**3,
            f"{output}: need at least 2 GiB free for two clean builds and packed consumers; set TMPDIR")
    require(not output.is_relative_to(root), "provenance scratch must be outside the checkout")
    for directory in (root, root / "javascript"):
        require(not os.path.lexists(directory / ".npmrc"), f"{directory / '.npmrc'} may contain secrets; use no project npmrc")
    baseline = inventory(root)
    schemas = schema_inputs()
    tree = output / "build"
    env = build_environment(tree)
    initial = baseline["source"] | baseline["packaging"]
    materialize(tree, {name: (root / name).read_bytes() for name in initial})
    equal(initial, files(tree, initial), "frozen build tree")
    versions = tools(tree)
    roots = crate_roots(tree, env)
    dependencies = crate_files(roots)
    info, = json.loads(command("npm", "pack", "./javascript", "--json", "--ignore-scripts=false",
                               "--pack-destination", str(output),
                               cwd=tree, env=env))
    artifact = output / info["filename"]
    require(artifact.parent == output and artifact.is_file(), "npm pack did not produce a local tarball")
    require(info["integrity"] == npm_integrity(artifact), "npm integrity does not match the tarball")
    package, corresponding = archive_contents(artifact, baseline["source"])
    packed_products = package_products(tree, package)
    equal(initial, files(tree, initial), "source used by npm pack")
    equal({"archive": digest(package["rocx-source.tar.gz"])},
          {"archive": file_digest(tree / "javascript/rocx-source.tar.gz")}, "npm corresponding source")

    # Rust may embed source paths or use them in crate identities. Replay at
    # the same absolute path, moving the first build (including its target)
    # out of the way. Only shipped source enters the fresh build directory.
    tree.rename(output / "packed-build")
    materialize(tree, corresponding)
    equal(baseline["source"], files(tree, baseline["source"]), "source extracted for replay")
    offline = env | {"CARGO_NET_OFFLINE": "true"}
    require(crate_roots(tree, offline) == roots, "Cargo dependencies changed for archive replay")
    subprocess.run(["bash", "javascript/build.sh"], cwd=tree, env=offline, check=True)
    equal(packed_products, package_products(tree, package), "raw and published Wasm/glue replay")
    require(crate_files(roots) == dependencies, "Cargo dependency bytes changed across builds")
    verify_checkout(root, {"inputs": baseline})
    require(tools(tree) == versions, "build tool versions changed during pack/replay")

    report = {
        "format": 1, "status": "built-and-replayed", "digest": "sha256",
        "checkout": str(root), "inputs": baseline, "schemas": schemas, "tools": versions,
        "registry_dependencies": dependencies,
        "tarball": {"name": artifact.name, "sha256": file_digest(artifact), "npm_integrity": info["integrity"]},
        "package_files": {name: digest(data) for name, data in package.items()},
        "products": packed_products, "rust_target": "wasm32-unknown-unknown",
    }
    write_report(output, report)
    (output / "package-name").write_text(artifact.name + "\n", encoding="utf-8")
    print(f"Frozen source and clean offline archive replay match exact release bytes: {output / 'provenance.json'}")


def check_install(output, report, consumer):
    require(consumer in {"node", "web"}, "consumer must be node or web")
    package_root = output / consumer / "node_modules/rocx-docx"
    installed = files(package_root, ["."])
    equal(report["package_files"], installed, f"{consumer} installed package vs tarball")
    return installed


def audit(output, report):
    require(report["format"] == 1 and report["digest"] == "sha256", "unknown provenance manifest format")
    artifact = output / report["tarball"]["name"]
    require(artifact.parent == output and file_digest(artifact) == report["tarball"]["sha256"], "tarball SHA-256 mismatch")
    require(npm_integrity(artifact) == report["tarball"]["npm_integrity"], "tarball npm integrity mismatch")
    package, _ = archive_contents(artifact, report["inputs"]["source"])
    equal(report["package_files"], {name: digest(data) for name, data in package.items()}, "packaged files")
    for name in ("packed-build", "build"):
        tree = output / name
        equal(report["inputs"]["source"], files(tree, report["inputs"]["source"]), f"{name} source")
        equal(report["products"], package_products(tree, package), f"{name} binary products")
    return {consumer: check_install(output, report, consumer) for consumer in ("node", "web")}


def verify_checkout(root, report):
    now = inventory(root)
    for key, before in report["inputs"].items():
        equal(before, now[key], f"checkout {key} inputs before/after")


def finish(root, output, report):
    verify_checkout(root, report)
    tree = output / "build"
    require(tools(tree) == report["tools"], "tool versions changed during consumer run")
    roots = crate_roots(tree, build_environment(tree) | {"CARGO_NET_OFFLINE": "true"})
    require(crate_files(roots) == report["registry_dependencies"], "Cargo dependency bytes changed during consumer run")
    report["installed"] = audit(output, report)
    require(report["installed"] == report.get("installed_before_tests"),
            "both consumers must be verified immediately after install")
    modules = output / "web/node_modules"
    report["consumer_tools"] = {
        name: json.loads((modules / name / "package.json").read_text(encoding="utf-8"))["version"]
        for name in ("playwright", "typescript", "vite")
    }
    report["consumer_tools"].update({
        name: metadata.version(name) for name in ("python-docx", "lxml")
    })
    report["consumer_tools"]["chromium"] = command("node", "--input-type=module", "-e", """
import { chromium } from 'playwright';
const browser = await chromium.launch();
try { console.log(browser.version()); } finally { await browser.close(); }
""", cwd=output / "web")
    runtime_entries = ["parity", "web/public/fixtures", "rust.docx", "python.docx",
                       "node.docx", "browser-dev.docx", "browser.docx"]
    report["verification_outputs"] = files(output, runtime_entries)
    require(schema_inputs() == report["schemas"], "schema input bytes changed during consumer run")
    verify_checkout(root, report)
    report["status"] = "qualified"
    write_report(output, report)
    print(f"Installed-package bytes and checkout stability verified: {output / 'provenance.json'}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("pack", "check-install", "finish", "verify"))
    parser.add_argument("output", type=Path)
    parser.add_argument("--checkout", type=Path)
    parser.add_argument("--consumer", choices=("node", "web"))
    args = parser.parse_args()
    output = args.output.resolve(strict=True)
    root = (args.checkout or Path.cwd()).resolve(strict=True)
    if args.mode == "pack":
        pack(root, output)
    else:
        report = json.loads((output / "provenance.json").read_text(encoding="utf-8"))
        if args.mode == "finish":
            finish(root, output, report)
        elif args.mode == "check-install":
            report.setdefault("installed_before_tests", {})[args.consumer] = check_install(output, report, args.consumer)
            write_report(output, report)
        else:
            require(report["status"] == "qualified", "consumer qualification did not complete")
            if args.checkout:
                verify_checkout(root, report)
            for consumer, manifest in audit(output, report).items():
                equal(report["installed"][consumer], manifest, f"{consumer} installed evidence")
            equal(report["verification_outputs"], files(output, report["verification_outputs"]),
                  "retained verification outputs")
            print(f"Provenance verified without rebuilding or installing: {output}")


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, ValueError, tarfile.TarError, subprocess.CalledProcessError) as error:
        sys.exit(f"packed provenance: {error}")
