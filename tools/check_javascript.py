"""Check packed consumer output: exact Rust ZIP bytes, readback, and OPC/schema."""

import argparse
from pathlib import Path

from check_examples import (
    exact_bytes,
    verify_content,
    verify_edited,
    verify_header_scopes,
    verify_javascript_workflow,
)
from validate_ooxml import Validator, default_directory

RUNTIMES = ("node", "browser-dev", "browser")
CATALOG_SUFFIXES = (
    ".docx", ".template.docx", ".edited.docx", ".header-source.docx",
    ".header-shared.docx", ".header-local.docx",
)
PORTABLE_ORACLES = (
    "text-layout", "linked-styles", "numbering-replacements", "drawings", "pictures",
    "tables", "tables-auto", "tables-layout",
    "fresh", "override", "edited", "structured",
)
EDITOR_OUTPUTS = ("body", "noop", "shared", "local", "relocated-out", "mixed", "mixedShared")


def verify_file(validator: Validator, path: Path, reference: bytes) -> bytes:
    data = path.read_bytes()
    try:
        digest = exact_bytes(reference, data)
        parts = validator.validate(path)
    except Exception as error:
        raise AssertionError(f"{path}: {error}") from error
    print(f"{path.name} ({path.parent.name}): {len(data)} bytes; SHA-256 {digest}; "
          f"{len(parts)} schema-covered parts and OPC relationships")
    return data


def verify_catalog(validator: Validator, parity: Path) -> None:
    references = parity / "catalog-references"
    native = [(references / f"all_features-rust{suffix}").read_bytes() for suffix in CATALOG_SUFFIXES]
    for language in ("rust", "python"):
        for suffix, data in zip(CATALOG_SUFFIXES, native, strict=True):
            verify_file(validator, references / f"all_features-{language}{suffix}", data)

    for runtime in RUNTIMES:
        final, template, edited, source, shared, local = (
            verify_file(validator, parity / runtime / f"catalog{suffix}", data)
            for suffix, data in zip(CATALOG_SUFFIXES, native, strict=True)
        )
        # Do not infer preservation or readback from container equality.
        verify_content(final, template)
        verify_edited(final, edited)
        verify_header_scopes(source, shared, local)
        print(f"{runtime}: six-artifact catalog, independent python-docx and untouched-part checks passed")


def verify_focused(validator: Validator, parity: Path) -> None:
    for name in PORTABLE_ORACLES:
        path = parity / "rust" / f"{name}.docx"
        reference = path.read_bytes()
        verify_file(validator, path, reference)
        for runtime in RUNTIMES:
            verify_file(validator, parity / runtime / path.name, reference)
    # Owner runtime checks the resolved native path variant, which has no
    # browser equivalent. Only the reference needs schema validation here.
    path = parity / "rust" / "image-path.docx"
    verify_file(validator, path, path.read_bytes())


def verify_editor(validator: Validator, parity: Path) -> None:
    reference = parity / "rust" / "editor"
    for name in ("base", "relocated"):
        path = reference / f"{name}.docx"
        print(f"editor/{path.name}: {len(validator.validate(path))} schema-covered parts and OPC relationships")
    for name in EDITOR_OUTPUTS:
        path = reference / f"native-{name}.docx"
        data = path.read_bytes()
        verify_file(validator, path, data)
        for runtime in RUNTIMES:
            verify_file(validator, parity / runtime / "editor" / f"{name}.docx", data)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_dir", type=Path)
    args = parser.parse_args()
    reference = (args.output_dir / "rust.docx").read_bytes()
    validator = Validator(default_directory())
    for name in ("rust", "python", *RUNTIMES):
        path = args.output_dir / f"{name}.docx"
        verify_javascript_workflow(verify_file(validator, path, reference), standalone=True)
    parity = args.output_dir / "parity"
    for name in ("imported-editor.docx", "no-margins.docx"):
        path = args.output_dir / "web" / "public" / "fixtures" / name
        print(f"{name}: {len(validator.validate(path))} schema-covered parts and OPC relationships")
    verify_focused(validator, parity)
    verify_catalog(validator, parity)
    verify_editor(validator, parity)


if __name__ == "__main__":
    main()
