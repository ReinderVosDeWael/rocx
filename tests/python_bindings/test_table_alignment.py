"""Inline table alignment requests do not change grids or paragraph alignment."""
import io
import pathlib

import docx
import pytest

from rv_docx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_table_alignment_tokens_and_omission_preserve_widths_and_text_alignment() -> None:
    values = [None, "left", "center", "right", "start", "end"]
    tables = [d.Table([d.TableRow([d.TableCell([d.Paragraph(text="unchanged", alignment="right")])])],
        column_widths=[1440], alignment=value) for value in values]
    snapshot = await d.Document([d.Section(tables)]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert [None if (n := table._tbl.find(f"{W}tblPr/{W}jc")) is None else n.get(f"{W}val") for table in output.tables] == values
    for table in output.tables:
        assert table._tbl.find(f"{W}tblGrid/{W}gridCol").get(f"{W}w") == "1440"
        assert table.cell(0, 0).text == "unchanged"
        assert table.cell(0, 0).paragraphs[0]._p.find(f"{W}pPr/{W}jc").get(f"{W}val") == "right"


async def test_table_alignment_references_deferred_fields_and_skips_hidden_producers() -> None:
    source = d.Table([], alignment=lambda: "end")
    hidden = d.Table([], alignment=lambda: (_ for _ in ()).throw(AssertionError("hidden alignment ran")), condition=False)
    table = d.Table([d.TableRow([d.TableCell([d.Paragraph(text="deferred")])])], alignment=source.deferred("alignment"))
    data = await d.Document([d.Section([table, hidden])]).to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert len(output.tables) == 1
    assert output.tables[0]._tbl.find(f"{W}tblPr/{W}jc").get(f"{W}val") == "end"


async def test_nested_table_alignment_is_independent_of_its_parent() -> None:
    nested = d.Table([d.TableRow([d.TableCell([d.Paragraph(text="nested")])])], alignment="start", column_widths=[1440])
    parent = d.Table([d.TableRow([d.TableCell([nested, d.Paragraph(text="after nested")])])], alignment="end", column_widths=[2880])
    data = await d.Document([d.Section([parent])]).to_bytes()
    output = docx.Document(io.BytesIO(data))
    parent = output.tables[0]
    child = parent.cell(0, 0).tables[0]
    assert parent._tbl.find(f"{W}tblPr/{W}jc").get(f"{W}val") == "end"
    assert child._tbl.find(f"{W}tblPr/{W}jc").get(f"{W}val") == "start"
    assert child._tbl.find(f"{W}tblGrid/{W}gridCol").get(f"{W}w") == "1440"
    assert parent.cell(0, 0).paragraphs[-1].text == "after nested"


async def test_authored_table_alignment_does_not_rewrite_template_tables(tmp_path: pathlib.Path) -> None:
    def table(alignment: str | None) -> d.Table:
        return d.Table([d.TableRow([d.TableCell([d.Paragraph(text="same")])])], alignment=alignment)
    path = tmp_path / "table-alignment.docx"
    path.write_bytes(await d.Document([d.Section([table("right")])]).to_bytes())
    data = await d.Document([d.Section([table("center")])]).to_bytes(d.DocumentTemplate(path))
    output = docx.Document(io.BytesIO(data))
    assert [table._tbl.find(f"{W}tblPr/{W}jc").get(f"{W}val") for table in output.tables] == ["right", "center"]


@pytest.mark.parametrize("token", ["", "CENTER", "justify", "both", "invalid"])
async def test_unsupported_table_alignment_is_rejected(token: str) -> None:
    table = d.Table([d.TableRow([d.TableCell([d.Paragraph(text="bad")])])], alignment=token)
    with pytest.raises(ValueError, match="table alignment"):
        await d.Document([d.Section([table])]).to_bytes()
