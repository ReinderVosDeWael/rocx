"""Chart authoring, independently read as a DOCX package."""
import io
from zipfile import ZipFile

import docx
from docx.enum.shape import WD_INLINE_SHAPE
from lxml import etree
import pytest

from rocx import declarative as d

NS = {
    "c": "http://schemas.openxmlformats.org/drawingml/2006/chart",
    "r": "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    "wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
}


def chart(**changes):
    fields = {"width": 144, "height": 72, "categories": ["North"], "values": [8]}
    return d.ColumnChartRun(**(fields | changes))


def document(*runs):
    return d.Document([d.Section([d.Paragraph(children=list(runs))])])


async def test_reads_deferred_chart_in_body_table_cell():
    calls = []

    async def values():
        calls.append("values")
        return [13.5, -4, 0]

    def excluded():
        pytest.fail("hidden chart data must not be evaluated")

    runs = [chart(categories=[" Café & tea ", "<South>", ""], values=values),
            chart(values=excluded, condition=False)]
    table = d.Table([d.TableRow([d.TableCell([d.Paragraph(children=runs)])])])
    snapshot = await d.Document([d.Section([table])]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    assert calls == ["values"]
    loaded = docx.Document(io.BytesIO(data))
    inline, = loaded.inline_shapes
    assert inline.type == WD_INLINE_SHAPE.CHART
    assert (inline.width, inline.height) == (1828800, 914400)
    reference, = inline._inline.iter(f"{{{NS['c']}}}chart")
    relation = loaded.part.rels[reference.get(f"{{{NS['r']}}}id")]
    assert not relation.is_external and relation.reltype == f"{NS['r']}/chart"
    root = etree.fromstring(relation.target_part.blob)
    labels = root.xpath("//c:strLit/c:pt/c:v", namespaces=NS)
    assert [label.text or "" for label in labels] == [" Café & tea ", "<South>", ""]
    assert root.xpath("//c:numLit/c:pt/c:v/text()", namespaces=NS) == ["13.5", "-4", "0"]
    body = etree.fromstring(loaded.part.blob)
    assert body.xpath("count(//w:tc//wp:inline)", namespaces=NS) == 1
    assert not root.xpath("//c:externalData", namespaces=NS)
    assert not relation.target_part.rels


@pytest.mark.parametrize(("changes", "diagnostic"), [
    ({"width": 0}, "chart dimensions"),
    ({"height": float("inf")}, "length must be finite"),
    ({"width": 200_000}, "chart dimensions"),
    ({"categories": [], "values": []}, "requires at least one category"),
    ({"values": [1, 2]}, "one value per category"),
    ({"values": [float("nan")]}, "must be finite"),
    ({"values": [float("inf")]}, "must be finite"),
    ({"categories": ["bad\0label"]}, r"categories\[0\]"),
])
async def test_chart_rejects_invalid_data(changes, diagnostic):
    with pytest.raises(ValueError, match=diagnostic):
        await document(chart(**changes)).to_bytes()


async def test_chart_rejects_other_stories_and_inline_containers():
    header = d.Header([d.Paragraph(children=[chart()])])
    with pytest.raises(ValueError, match="only in body paragraphs"):
        await d.Document([d.Section([], headers={"default": header})]).to_bytes()
    for container, message in [
        (d.Field(d.FieldInstruction.page(), children=[chart()]), "cached field results"),
        (d.Hyperlink("https://example.test", [chart()]), "hyperlink children"),
        (d.InlineContentControl([chart()]), "inline content control children"),
    ]:
        with pytest.raises(ValueError, match=message):
            await document(container).to_bytes()


async def test_appending_charts_preserves_existing_chart_part_and_drawing_id(tmp_path):
    from test_notes import rewrite

    original = await document(chart()).to_bytes()
    template = tmp_path / "existing-chart.docx"

    def reserve_id(root):
        docpr, = root.xpath("//wp:inline/wp:docPr", namespaces=NS)
        docpr.set("id", "900")

    template.write_bytes(rewrite(original, "word/document.xml", reserve_id))
    added = await document(chart(values=[9]), chart(values=[9])).to_bytes(d.DocumentTemplate(template))
    with ZipFile(io.BytesIO(original)) as before, ZipFile(io.BytesIO(added)) as after:
        assert before.read("word/charts/chart1.xml") == after.read("word/charts/chart1.xml")
        chart_parts = [f"word/charts/chart{i}.xml" for i in range(1, 4)]
        assert [etree.fromstring(after.read(part)).xpath("//c:numLit/c:pt/c:v/text()", namespaces=NS)
                for part in chart_parts] == [["8"], ["9"], ["9"]]
        root = etree.fromstring(after.read("word/document.xml"))
    assert root.xpath("//wp:inline/wp:docPr/@id", namespaces=NS) == ["900", "901", "902"]
    assert len(set(root.xpath("//c:chart/@r:id", namespaces=NS))) == 3
