"""Check packed consumer output: exact Rust ZIP bytes, readback, and OPC/schema."""

import argparse
import asyncio
import io
import zipfile
from pathlib import Path

import docx

from check_examples import (
    CATALOG_SUFFIXES,
    exact_bytes,
    verify_catalog_readback,
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
EDITOR_OUTPUTS = ("body", "cell", "mixedBody", "noop", "removal", "range", "shared", "local",
                  "firstShared", "firstLocal", "relocated-out", "mixed", "mixedShared",
                  "blocks", "inline", "tableStructure", "styleEdits", "controlEdits",
                  "tracked", "reviewed", "moved", "moveAccepted", "combined", "numberingEdits")


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
    native = {suffix: (references / f"all_features-rust{suffix}").read_bytes() for suffix in CATALOG_SUFFIXES}
    for language in ("rust", "python"):
        expected = {f"all_features-{language}{suffix}" for suffix in native}
        assert {p.name for p in references.glob(f"all_features-{language}*.docx")} == expected
        for suffix, data in native.items():
            verify_file(validator, references / f"all_features-{language}{suffix}", data, language=language)

    for runtime in RUNTIMES:
        directory = parity / runtime
        assert {p.name for p in directory.glob("catalog*.docx")} == {f"catalog{s}" for s in native}
        files = {s: verify_file(validator, directory / f"catalog{s}", data, language=runtime)
                 for s, data in native.items()}
        verify_catalog_readback(files)
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


async def python_mixed_body(source: Path, output: Path) -> None:
    from rocx import declarative as d
    from rocx.editor import CellAlignmentEdit, ImportedDocument, ParagraphInsertion

    doc = ImportedDocument.open(source.read_bytes())
    paragraph = await d.Paragraph(text="inserted paragraph").resolve()
    preview = doc.preview(
        [],
        paragraph_insertions=[ParagraphInsertion(doc.body[0].anchor, paragraph)],
        cell_alignments=[CellAlignmentEdit(doc.body[5].anchor, 0, 0, "center")],
        run_removals=[doc.body[2].inline_structure[0].anchor],
    )
    assert preview.paragraph_insertions == [0]
    assert [(c.body_index, c.row_index, c.cell_index) for c in preview.cell_alignment_changes] == [(5, 0, 0)]
    assert [(r.body_index, r.inline_index, r.before) for r in preview.run_removals] == [(2, 0, "unset alignment")]
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(preview.to_docx())


async def python_blocks(source: Path) -> bytes:
    from rocx import declarative as d
    from rocx.editor import BlockEdit, ImportedDocument

    doc = ImportedDocument.open(source.read_bytes())
    p = await d.Paragraph(text="inserted paragraph").resolve()
    table = await d.Table([d.TableRow([d.TableCell([d.Paragraph(text=t)]) for t in ("next A", "next B")])]).resolve()
    batch = doc.preview([], block_edits=[
        BlockEdit.keep_with_next(doc.body[0].anchor, False),
        BlockEdit.replace(doc.body[1].anchor, p),
        BlockEdit.replace_section(doc.body[4].anchor, [p, p, table]),
    ])
    assert batch.block_changes == [0, 1, 4]
    intermediate = ImportedDocument.open(batch.to_docx())
    return intermediate.preview([], block_edits=[BlockEdit.remove(intermediate.body[5].anchor)]).to_docx()


async def python_table_structure(source: Path, output: Path) -> None:
    from rocx import declarative as d
    from rocx.editor import ImportedDocument, TableStructureEdit as E

    data = source.read_bytes()
    paragraph = await d.Paragraph(text="inserted paragraph").resolve()
    nested = await d.Table([d.TableRow([
        d.TableCell([d.Paragraph(text=t)]) for t in ("next A", "next B")
    ])]).resolve()
    for build in (lambda a: E.insert_row(a, 1), lambda a: E.remove_row(a, 1),
                  lambda a: E.insert_column(a, 1), lambda a: E.remove_column(a, 1),
                  lambda a: E.insert_cell(a, 0, 1), lambda a: E.remove_cell(a, 0, 1),
                  lambda a: E.replace_cell(a, 0, 1, [paragraph, nested])):
        doc = ImportedDocument.open(data)
        data = doc.preview([], table_structure=[build(doc.body[5].anchor)]).to_docx()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(data)


def python_combined(source: Path) -> bytes:
    from rocx.editor import BlockEdit, ImportedDocument, ResourceEdit, TextRangeEdit

    original = source.read_bytes()
    doc = ImportedDocument.open(original)
    preview = doc.preview(
        [], text_ranges=[TextRangeEdit(doc.body[1].anchor, 0, 1, "X")],
        block_edits=[BlockEdit.keep_with_next(doc.body[4].anchor, True)],
        resources=[ResourceEdit.document_default("tabStops", [(720, "right", "dot")]),
                   ResourceEdit.core_title("Combined editor")],
    )
    assert preview.block_changes == [4]
    assert [(c.body_index, c.before, c.after) for c in preview.run_text_changes] == [
        (1, "unset alignment", "Xnset alignment")]
    data = preview.to_docx()
    readback = docx.Document(io.BytesIO(data))
    assert readback.paragraphs[1].text == "Xnset alignment"
    assert readback.paragraphs[4].text == "next section"
    assert readback.paragraphs[4].paragraph_format.keep_with_next is True
    assert readback.core_properties.title == "Combined editor"
    with zipfile.ZipFile(io.BytesIO(original)) as before, zipfile.ZipFile(io.BytesIO(data)) as after:
        assert set(before.namelist()) == set(after.namelist())
        assert {p for p in before.namelist() if before.read(p) != after.read(p)} == {
            "word/document.xml", "docProps/core.xml", "word/styles.xml"}
    return data


def verify_editor(validator: Validator, parity: Path) -> None:
    reference = parity / "rust" / "editor"
    for name in ("base", "relocated", "range-source", "styles-source", "control-source"):
        path = reference / f"{name}.docx"
        print(f"editor/{path.name}: {len(validator.validate(path))} schema-covered parts and OPC relationships")
    assert {p.name for p in reference.glob("native-*.docx")} == {f"native-{n}.docx" for n in EDITOR_OUTPUTS}
    for name in EDITOR_OUTPUTS:
        path = reference / f"native-{name}.docx"
        data = path.read_bytes()
        verify_file(validator, path, data, language="Rust")
        if name == "tableStructure":
            python = parity / "python" / "editor" / "tableStructure.docx"
            asyncio.run(python_table_structure(reference / "native-body.docx", python))
            verify_file(validator, python, data, language="Python")
        if name == "blocks":
            python = parity / "python" / "editor" / "blocks.docx"
            python.parent.mkdir(parents=True, exist_ok=True)
            python.write_bytes(asyncio.run(python_blocks(reference / "base.docx")))
            verify_file(validator, python, data, language="Python")
        if name == "combined":
            python = parity / "python" / "editor" / "combined.docx"
            python.parent.mkdir(parents=True, exist_ok=True)
            python.write_bytes(python_combined(reference / "base.docx"))
            verify_file(validator, python, data, language="Python")
        if name == "mixedBody":
            python = parity / "python" / "editor" / "mixedBody.docx"
            asyncio.run(python_mixed_body(reference / "native-body.docx", python))
            verify_file(validator, python, data, language="Python")
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
