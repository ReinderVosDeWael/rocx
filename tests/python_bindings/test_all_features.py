"""The visual catalogs must retain semantic coverage and exact ZIP byte parity."""
import importlib.util
import io
import pathlib
import zipfile

from lxml import etree
import pytest
from rocx.editor import (
    BodyParagraph, BodyTable, ImportedDocument, ImportedHeaderParagraph, ImportedHyperlink,
    ImportedRun,
)

ROOT = pathlib.Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("check_examples", ROOT / "tools/check_examples.py")
checks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checks)


@pytest.fixture(scope="module")
def catalogs(tmp_path_factory):
    """Generate independently through the two public authoring APIs."""
    return checks.generate(tmp_path_factory.mktemp("catalogs"))


def test_all_features_catalogs_have_exact_byte_parity_and_expected_content(catalogs) -> None:
    rust, python = catalogs
    checks.verify_inventory()
    assert checks.exact_bytes(rust.read_bytes(), python.read_bytes())
    checks.exact_bytes(rust.with_suffix(".template.docx").read_bytes(), python.with_suffix(".template.docx").read_bytes())
    checks.verify_content(rust.read_bytes(), rust.with_suffix(".template.docx").read_bytes())
    edited = rust.with_suffix(".edited.docx").read_bytes()
    checks.exact_bytes(edited, python.with_suffix(".edited.docx").read_bytes())
    checks.verify_edited(rust.read_bytes(), edited)
    reopened = ImportedDocument.open(edited)
    edited_body = reopened.body
    target_index = next(i for i, block in enumerate(edited_body)
                        if isinstance(block, BodyParagraph) and block.text == checks.EDIT_LABEL)
    table = edited_body[target_index + 1]
    assert isinstance(table, BodyTable)
    assert len(table.children) == 1
    assert len(table.children[0].children) == 2
    assert [(item.kind, item.value) for item in table.children[0].children[0].blocks[0].inline_items] == [
        ("text", checks.EDIT_TABLE_LABEL),
    ]
    first, second = table.children[0].children
    label, = first.blocks[0].inline_structure
    prefix, suffix = second.blocks[0].inline_structure
    assert all(isinstance(node, ImportedRun) for node in (label, prefix, suffix))
    assert label.anchor != prefix.anchor
    assert [[item.value for item in reopened.inline_at(run.anchor).items]
            for run in (prefix, suffix)] == [["Fixed-width "], ["second column"]]
    for data in (rust.read_bytes(), edited):
        imported = ImportedDocument.open(data)
        target, = (
            block for block in imported.body
            if isinstance(block, BodyParagraph) and block.text == checks.EDIT_LABEL
        )
        assert [(item.kind, item.value) for item in target.inline_items] == [
            ("text", checks.EDIT_LABEL),
        ]
        sample, = (block for block in imported.body if isinstance(block, BodyParagraph)
                   and block.text == checks.INLINE_LABEL + "".join(checks.INLINE_LINK_RUNS))
        label, link = sample.inline_structure
        assert isinstance(label, ImportedRun) and isinstance(link, ImportedHyperlink)
        assert imported.inline_at(link.anchor).bookmark == "CatalogBookmark"
        assert all(isinstance(child, ImportedRun) for child in link.children)
        assert [[item.value for item in imported.inline_at(run.anchor).items]
                for run in link.children] == [[text] for text in checks.INLINE_LINK_RUNS]
    suffixes = [".header-source.docx", ".header-shared.docx", ".header-local.docx"]
    for suffix in suffixes:
        checks.exact_bytes(rust.with_suffix(suffix).read_bytes(), python.with_suffix(suffix).read_bytes())
        imported = ImportedDocument.open(rust.with_suffix(suffix).read_bytes())
        expected = [(0, 2), (2, 3)] if suffix == ".header-source.docx" else [(0, 3), (3, 4)]
        assert [(s.body_start, s.body_end) for s in imported.sections] == expected
        first, second = imported.default_headers
        anchors = []
        for header in (first, second):
            paragraph = header.blocks[1]
            assert isinstance(paragraph, ImportedHeaderParagraph)
            label, link = paragraph.inline_structure
            assert isinstance(label, ImportedRun) and isinstance(link, ImportedHyperlink)
            assert label.items[0].value == checks.HEADER_INLINE_LABEL
            assert imported.inline_at(link.anchor).relationship_id == link.relationship_id
            assert [[item.value for item in imported.inline_at(run.anchor).items]
                    for run in link.children] == [[text] for text in checks.HEADER_LINK_RUNS]
            anchors.append((label.anchor, link.anchor, *(run.anchor for run in link.children)))
        assert (first.part_name == second.part_name) == (suffix != ".header-local.docx")
        assert all((a == b) == (first.part_name == second.part_name)
                   for a, b in zip(*anchors, strict=True))
    checks.verify_header_scopes(*(rust.with_suffix(suffix).read_bytes() for suffix in suffixes))


def test_header_scope_coverage_rejects_wrong_preview(catalogs) -> None:
    rust, _ = catalogs
    source, shared, local = (rust.with_suffix(suffix).read_bytes() for suffix in [
        ".header-source.docx", ".header-shared.docx", ".header-local.docx"
    ])
    with pytest.raises(AssertionError):
        checks.verify_header_scopes(source, local, shared)
    with pytest.raises(AssertionError):
        checks.verify_header_scopes(source, source, local)
    # Keep the modified shared header but strip only the body edits.
    header_only = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(source)) as before, \
            zipfile.ZipFile(io.BytesIO(shared)) as after, \
            zipfile.ZipFile(header_only, "w") as broken:
        for name in after.namelist():
            broken.writestr(name, (before if name == "word/document.xml" else after).read(name))
    with pytest.raises(AssertionError):
        checks.verify_header_scopes(source, header_only.getvalue(), local)


@pytest.mark.parametrize("change", ["merge_runs", "relationship_target"])
def test_header_inline_coverage_rejects_changed_structure(catalogs, change) -> None:
    rust, _ = catalogs
    source = rust.with_suffix(".header-source.docx").read_bytes()
    part = ImportedDocument.open(source).default_headers[0].part_name.lstrip("/")
    directory, filename = part.rsplit("/", 1)
    with zipfile.ZipFile(io.BytesIO(source)) as package:
        root = etree.fromstring(package.read(part))
        rels = etree.fromstring(package.read(f"{directory}/_rels/{filename}.rels"))
    if change == "merge_runs":
        link, = root.xpath("/w:hdr/w:p/w:hyperlink", namespaces=checks.NS)
        text = link[0].find("w:t", checks.NS)
        text.text += link[1].find("w:t", checks.NS).text
        link.remove(link[1])
        assert "".join(link.itertext()) == "".join(checks.HEADER_LINK_RUNS)
    else:
        rel, = rels
        rel.set("Target", "https://example.invalid/other")
    with pytest.raises(AssertionError):
        checks.verify_header_inline_structure(root, rels)


def test_edit_coverage_rejects_unchanged_catalog(catalogs) -> None:
    rust, _ = catalogs
    with pytest.raises(AssertionError):
        checks.verify_edited(rust.read_bytes(), rust.read_bytes())


def test_inline_coverage_rejects_merged_runs_with_identical_text(catalogs) -> None:
    rust, _ = catalogs
    with zipfile.ZipFile(rust) as source:
        root = etree.fromstring(source.read("word/document.xml"))
    link, = root.xpath("/w:document/w:body/w:p[w:r/w:t=$label]/w:hyperlink",
                       namespaces=checks.NS, label=checks.INLINE_LABEL)
    text = link[0].find("w:t", checks.NS)
    text.text += link[1].find("w:t", checks.NS).text
    link.remove(link[1])
    assert "".join(link.itertext()) == "".join(checks.INLINE_LINK_RUNS)
    with pytest.raises(AssertionError):
        checks.verify_inline_structure(root)


@pytest.mark.parametrize("change", ["position", "shape", "width", "bold", "source", "merge_runs"])
def test_edit_coverage_rejects_changed_inserted_table(catalogs, change) -> None:
    rust, _ = catalogs
    original = rust.read_bytes()
    edited = rust.with_suffix(".edited.docx").read_bytes()
    output = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(edited if change != "source" else original)) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(data)
                if change == "source":
                    with zipfile.ZipFile(io.BytesIO(edited)) as edited_package:
                        edited_root = etree.fromstring(edited_package.read(name))
                    table = edited_root.xpath("/w:document/w:body/w:tbl[.//w:t[text()=$label]]",
                                              namespaces=checks.NS, label=checks.EDIT_TABLE_LABEL)[0]
                    root.xpath("/w:document/w:body", namespaces=checks.NS)[0].insert(0, table)
                else:
                    table = root.xpath("/w:document/w:body/w:tbl[.//w:t[text()=$label]]",
                                       namespaces=checks.NS, label=checks.EDIT_TABLE_LABEL)[0]
                if change == "position":
                    table.getparent().insert(0, table)
                elif change == "shape":
                    row = table.xpath("w:tr", namespaces=checks.NS)[0]
                    row.remove(row.xpath("w:tc", namespaces=checks.NS)[1])
                elif change == "width":
                    table.xpath("w:tblGrid/w:gridCol", namespaces=checks.NS)[0].set(
                        f"{{{checks.NS['w']}}}w", "100")
                elif change == "bold":
                    bold = table.xpath(".//w:rPr/w:b", namespaces=checks.NS)[0]
                    bold.getparent().remove(bold)
                elif change == "merge_runs":
                    paragraph, = table.xpath("w:tr/w:tc[2]/w:p", namespaces=checks.NS)
                    first, second = paragraph.xpath("w:r", namespaces=checks.NS)
                    text = first.find("w:t", checks.NS)
                    text.text += second.find("w:t", checks.NS).text
                    paragraph.remove(second)
                    assert "".join(paragraph.itertext()) == "Fixed-width second column"
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        if change == "source":
            checks.verify_edited(output.getvalue(), edited)
        else:
            checks.verify_edited(original, output.getvalue())


def test_builder_inventory_rejects_a_missing_setter(tmp_path, monkeypatch) -> None:
    source_dir = tmp_path / "src/document"
    source_dir.mkdir(parents=True)
    tests_dir = tmp_path / "tests"
    tests_dir.mkdir()
    for name in ["authoring_builders.rs", "run_format_builders.rs"]:
        (tests_dir / name).write_text((ROOT / "tests" / name).read_text())
    source = (ROOT / "src/document/values.rs").read_text()
    (source_dir / "values.rs").write_text(source.replace("    with_bold => bold: optional<bool>,\n", "", 1))
    monkeypatch.setattr(checks, "ROOT", tmp_path)
    with pytest.raises(AssertionError, match="Missing RunFormat builder fields"):
        checks.verify_rust_builder_inventory()


@pytest.mark.parametrize("property", ["b", "i", "typeface"])
def test_content_check_rejects_changed_shape_text_style(catalogs, property) -> None:
    rust, _ = catalogs
    output = io.BytesIO()
    with zipfile.ZipFile(rust) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(data)
                properties = root.xpath("//a:txBody/a:p/a:r/a:rPr", namespaces=checks.NS)[0]
                if property == "typeface":
                    properties.find("a:latin", checks.NS).set("typeface", "Wrong font")
                else:
                    properties.set(property, "1" if property == "b" else "0")
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        checks.verify_content(output.getvalue(), rust.with_suffix(".template.docx").read_bytes())


def test_content_check_rejects_changed_text_box_extent(catalogs) -> None:
    rust, _ = catalogs
    output = io.BytesIO()
    with zipfile.ZipFile(rust) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(data)
                extent = root.xpath("//a:graphicData/*[local-name()='lockedCanvas']/a:txSp/a:xfrm/a:ext", namespaces=checks.NS)[0]
                extent.set("cx", "100")
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        checks.verify_content(output.getvalue(), rust.with_suffix(".template.docx").read_bytes())


def test_exact_byte_check_rejects_container_only_differences(catalogs) -> None:
    rust, _ = catalogs
    original = rust.read_bytes()
    output = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(original)) as source, zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as target:
        for name in source.namelist():
            target.writestr(name, source.read(name))
    with pytest.raises(AssertionError, match="ZIP container metadata/compression only"):
        checks.exact_bytes(original, output.getvalue())


def test_content_check_rejects_changed_chart_values(catalogs) -> None:
    from test_notes import rewrite

    rust, _ = catalogs

    def change_value(root):
        value = root.xpath("//c:numLit/c:pt[@idx='1']/c:v", namespaces=checks.NS)[0]
        value.text = "12.5"

    changed = rewrite(rust.read_bytes(), "word/charts/chart1.xml", change_value)
    with pytest.raises(AssertionError):
        checks.verify_content(changed, rust.with_suffix(".template.docx").read_bytes())


def test_content_check_rejects_changed_fraction_operand(catalogs) -> None:
    from test_notes import rewrite

    rust, _ = catalogs

    def change_operand(root):
        operand, = root.xpath("//m:oMath/m:f/m:den/m:r/m:t", namespaces=checks.NS)
        operand.text = "3"

    changed = rewrite(rust.read_bytes(), "word/document.xml", change_operand)
    with pytest.raises(AssertionError):
        checks.verify_content(changed, rust.with_suffix(".template.docx").read_bytes())


def test_content_check_rejects_changed_custom_boolean(catalogs) -> None:
    from test_notes import rewrite

    rust, _ = catalogs

    def change_value(root):
        value, = root.xpath("cust:property[@name='Published']/vt:bool", namespaces=checks.NS)
        value.text = "true"

    changed = rewrite(rust.read_bytes(), "docProps/custom.xml", change_value)
    with pytest.raises(AssertionError):
        checks.verify_content(changed, rust.with_suffix(".template.docx").read_bytes())


def test_content_check_rejects_changed_floating_text_box_position(catalogs) -> None:
    from test_notes import rewrite

    rust, _ = catalogs

    def change_position(root):
        offset, = root.xpath("//wp:anchor[.//a:txSp]/wp:positionV/wp:posOffset", namespaces=checks.NS)
        offset.text = "0"

    changed = rewrite(rust.read_bytes(), "word/document.xml", change_position)
    with pytest.raises(AssertionError):
        checks.verify_content(changed, rust.with_suffix(".template.docx").read_bytes())


def test_content_check_rejects_losing_explicit_false_from_a_builder(catalogs) -> None:
    rust, _ = catalogs
    output = io.BytesIO()
    with zipfile.ZipFile(rust) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(data)
                node = root.xpath("//w:r[w:t[text()='Builder bold disabled, italic enabled.']]/w:rPr/w:b", namespaces=checks.NS)[0]
                node.getparent().remove(node)
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        checks.verify_content(output.getvalue(), rust.with_suffix(".template.docx").read_bytes())


def test_content_check_detects_a_missing_feature_even_if_both_languages_agree(catalogs) -> None:
    rust, _ = catalogs
    output = io.BytesIO()
    with zipfile.ZipFile(rust) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(data)
                for node in root.xpath("//w:tcPr/w:textDirection", namespaces=checks.NS):
                    node.getparent().remove(node)
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        checks.verify_content(output.getvalue(), rust.with_suffix(".template.docx").read_bytes())


@pytest.mark.parametrize("element, value", [
    ("isLgl", "1"), ("suff", "nothing"), ("lvlRestart", "1"), ("lvlJc", "left"),
    ("abstractNumId", "1"), ("rPr/w:b", "1"), ("pStyle", "Heading1"),
])
def test_content_check_rejects_a_changed_numbering_request(catalogs, element: str, value: str) -> None:
    rust, _ = catalogs
    output = io.BytesIO()
    with zipfile.ZipFile(rust) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/numbering.xml":
                root = etree.fromstring(data)
                path = ("w:num[@w:numId='62']/w:abstractNumId" if element == "abstractNumId"
                        else f"w:num[@w:numId='60']/w:lvlOverride[@w:ilvl='0']/w:lvl/w:{element}")
                node = root.xpath(path, namespaces=checks.NS)[0]
                node.set(f"{{{checks.NS['w']}}}val", value)
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        checks.verify_content(output.getvalue(), rust.with_suffix(".template.docx").read_bytes())


def test_content_check_rejects_a_changed_table_alignment(catalogs) -> None:
    rust, _ = catalogs
    output = io.BytesIO()
    with zipfile.ZipFile(rust) as source, zipfile.ZipFile(output, "w") as target:
        for name in source.namelist():
            data = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(data)
                node = root.xpath("//w:tbl[.//w:t[text()='Table alignment: end']]/w:tblPr/w:jc", namespaces=checks.NS)[0]
                node.set(f"{{{checks.NS['w']}}}val", "left")
                data = etree.tostring(root)
            target.writestr(name, data)
    with pytest.raises(AssertionError):
        checks.verify_content(output.getvalue(), rust.with_suffix(".template.docx").read_bytes())
