"""Inline OMML fraction authoring and placement."""
import io
from zipfile import ZipFile

from lxml import etree
import pytest

from rocx import declarative as d

NS = {
    "m": "http://schemas.openxmlformats.org/officeDocument/2006/math",
    "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
}


def document(*runs):
    return d.Document([d.Section([d.Paragraph(children=list(runs))])])


def body(data):
    with ZipFile(io.BytesIO(data)) as package:
        return etree.fromstring(package.read("word/document.xml"))


async def test_fraction_deferred_snapshot_and_table_placement():
    calls = []

    async def numerator():
        calls.append("numerator")
        return " café & tea "

    def hidden():
        pytest.fail("hidden fraction operands must not be evaluated")

    table = d.Table([d.TableRow([d.TableCell([d.Paragraph(children=[
        d.InlineFraction(numerator=numerator, denominator="<2>"),
        d.InlineFraction(numerator=hidden, denominator=hidden, condition=False),
    ])])])])
    snapshot = await d.Document([d.Section([table])]).resolve()
    first = await snapshot.to_bytes()
    assert first == await snapshot.to_bytes()
    assert calls == ["numerator"]
    root = body(first)
    fractions = root.xpath("//w:tc/w:p/m:oMath/m:f", namespaces=NS)
    assert len(fractions) == 1
    fraction = fractions[0]
    assert [etree.QName(child).localname for child in fraction] == ["num", "den"]
    assert fraction.xpath("m:num/m:r/m:t/text()", namespaces=NS) == [" café & tea "]
    assert fraction.xpath("m:den/m:r/m:t/text()", namespaces=NS) == ["<2>"]


@pytest.mark.parametrize("value", ["", "a\nb", "a\rb", "bad\0text"])
@pytest.mark.parametrize("operand", ["numerator", "denominator"])
async def test_fraction_rejects_invalid_operand(value, operand):
    fields = {"numerator": "1", "denominator": "2"}
    fields[operand] = value
    with pytest.raises(ValueError):
        await document(d.InlineFraction(**fields)).to_bytes()


async def test_fraction_rejects_other_stories_and_inline_containers():
    def fraction():
        return d.InlineFraction(numerator="1", denominator="2")

    header = d.Header([d.Paragraph(children=[fraction()])])
    with pytest.raises(ValueError):
        await d.Document([d.Section([], headers={"default": header})]).to_bytes()
    for container in [
        d.Field(d.FieldInstruction.page(), children=[fraction()]),
        d.Hyperlink("https://example.test", [fraction()]),
        d.InlineContentControl([fraction()]),
    ]:
        with pytest.raises(ValueError):
            await document(container).to_bytes()


async def test_appending_fraction_preserves_imported_equation(tmp_path):
    original = await document(d.InlineFraction(numerator="1", denominator="2")).to_bytes()
    template = tmp_path / "equation.docx"
    template.write_bytes(original)
    result = await document(d.InlineFraction(numerator="3", denominator="4")).to_bytes(
        d.DocumentTemplate(template)
    )
    old, = body(original).xpath("//m:oMath", namespaces=NS)
    equations = body(result).xpath("//m:oMath", namespaces=NS)
    assert len(equations) == 2
    assert etree.tostring(old, method="c14n") == etree.tostring(equations[0], method="c14n")
    assert equations[1].xpath("m:f/m:num/m:r/m:t/text()", namespaces=NS) == ["3"]
