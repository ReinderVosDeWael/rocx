"""The visual catalogs must retain semantic coverage and exact ZIP byte parity."""
import importlib.util
import io
import pathlib
import zipfile

from lxml import etree
import pytest

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


def test_edit_coverage_rejects_unchanged_catalog(catalogs) -> None:
    rust, _ = catalogs
    with pytest.raises(AssertionError):
        checks.verify_edited(rust.read_bytes(), rust.read_bytes())


@pytest.mark.parametrize("change", ["position", "shape", "width", "bold", "source"])
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
