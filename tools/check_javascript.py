"""Check packed consumer output: exact Rust ZIP bytes, readback, and OPC/schema."""

import argparse
import io
import zipfile
from pathlib import Path

import docx

from check_examples import (
    CATALOG_SUFFIXES,
    INLINE_LINK_RUNS,
    exact_bytes,
    verify_content,
    verify_edited,
    verify_header_scopes,
    verify_javascript_workflow,
    verify_plain_workflow,
)
from validate_ooxml import Validator, default_directory

RUNTIMES = ("node", "browser-dev", "browser")
PORTABLE_ORACLES = (
    "text-layout", "linked-styles", "numbering-replacements", "drawings", "pictures",
    "tables", "tables-auto", "tables-layout",
    "fresh", "override", "edited", "structured",
)
EDITOR_OUTPUTS = ("body", "noop", "shared", "local", "relocated-out", "mixed", "mixedShared")


def verify_file(validator: Validator, path: Path, reference: bytes, *, language: str) -> bytes:
    data = path.read_bytes()
    try:
        digest = exact_bytes(reference, data, language=language)
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
            verify_file(validator, references / f"all_features-{language}{suffix}", data, language=language)

    for runtime in RUNTIMES:
        final, template, edited, source, shared, local, run_text = (
            verify_file(validator, parity / runtime / f"catalog{suffix}", data, language=runtime)
            for suffix, data in zip(CATALOG_SUFFIXES, native, strict=True)
        )
        # Do not infer preservation or readback from container equality.
        verify_content(final, template)
        verify_edited(final, edited)
        verify_header_scopes(source, shared, local)
        paragraphs = [p.text for p in docx.Document(io.BytesIO(run_text)).paragraphs]
        anchor = paragraphs.index("Imported editor: replaced this run by its anchor. " + "".join(INLINE_LINK_RUNS))
        assert paragraphs[anchor + 1] == "Imported editor: inserted paragraph."
        with zipfile.ZipFile(io.BytesIO(final)) as source_zip, zipfile.ZipFile(io.BytesIO(run_text)) as result_zip:
            assert set(source_zip.namelist()) == set(result_zip.namelist())
            assert all(source_zip.read(name) == result_zip.read(name)
                       for name in source_zip.namelist() if name != "word/document.xml")
        print(f"{runtime}: catalog, independent python-docx and untouched-part checks passed")


def verify_focused(validator: Validator, parity: Path) -> None:
    for name in PORTABLE_ORACLES:
        path = parity / "rust" / f"{name}.docx"
        reference = path.read_bytes()
        verify_file(validator, path, reference, language="Rust")
        for runtime in RUNTIMES:
            verify_file(validator, parity / runtime / path.name, reference, language=runtime)
    # Focused owner suites already compare the installed bytes to these files.
    # The unparsed cases deliberately exercise non-date core metadata; the
    # validator checks covered document parts and OPC, not W3CDTF date text.
    timestamps = (f"{kind}-{mode}" for kind in ("fractional", "offset", "unparsed")
                  for mode in ("snapshot", "override", "template"))
    for name in ("image-path", *timestamps):
        path = parity / "rust" / f"{name}.docx"
        verify_file(validator, path, path.read_bytes(), language="Rust")


def verify_declarative(validator: Validator, parity: Path, workflow: bytes) -> None:
    record = (parity / "node" / "declarative-record.docx").read_bytes()
    for runtime in RUNTIMES:
        directory = parity / runtime
        verify_file(validator, directory / "legacy-workflow.docx", workflow, language=runtime)
        for name in ("declarative-record", "declarative"):
            verify_file(validator, directory / f"{name}.docx", record, language=runtime)


def verify_plain(validator: Validator, parity: Path) -> None:
    reference = (parity / "rust" / "plain.docx").read_bytes()
    for language in ("rust", "python", "node"):
        data = verify_file(validator, parity / language / "plain.docx", reference, language=language)
        verify_plain_workflow(data)


def verify_editor(validator: Validator, parity: Path) -> None:
    reference = parity / "rust" / "editor"
    for name in ("base", "relocated"):
        path = reference / f"{name}.docx"
        print(f"editor/{path.name}: {len(validator.validate(path))} schema-covered parts and OPC relationships")
    for name in EDITOR_OUTPUTS:
        path = reference / f"native-{name}.docx"
        data = path.read_bytes()
        verify_file(validator, path, data, language="Rust")
        for runtime in RUNTIMES:
            verify_file(validator, parity / runtime / "editor" / f"{name}.docx", data, language=runtime)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_dir", type=Path)
    args = parser.parse_args()
    reference = (args.output_dir / "rust.docx").read_bytes()
    validator = Validator(default_directory())
    for name in ("rust", "python", *RUNTIMES):
        path = args.output_dir / f"{name}.docx"
        verify_javascript_workflow(verify_file(validator, path, reference, language=name), standalone=True)
    parity = args.output_dir / "parity"
    for name in ("imported-editor.docx", "no-margins.docx"):
        path = args.output_dir / "web" / "public" / "fixtures" / name
        print(f"{name}: {len(validator.validate(path))} schema-covered parts and OPC relationships")
    verify_focused(validator, parity)
    verify_declarative(validator, parity, reference)
    verify_plain(validator, parity)
    verify_catalog(validator, parity)
    verify_editor(validator, parity)


if __name__ == "__main__":
    main()
