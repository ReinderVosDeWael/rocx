"""Table-look requests activate regions without computing effective styles."""
import io
import itertools
import pathlib

import docx

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_omitted_look_and_empty_record_keep_existing_library_defaults() -> None:
    outputs = []
    for look in [None, d.TableLook()]:
        data = await d.Document([d.Section([d.Table([d.TableRow([d.TableCell([d.Paragraph(text="default look")])])], look=look)])], timestamp="2026-11-19T00:00:00Z").to_bytes()
        outputs.append(data)
        node = docx.Document(io.BytesIO(data)).tables[0]._tbl.find(f"{W}tblPr/{W}tblLook")
        assert dict(node.attrib) == {f"{W}firstColumn": "1", f"{W}firstRow": "1", f"{W}lastColumn": "0", f"{W}lastRow": "0", f"{W}noHBand": "0", f"{W}noVBand": "1", f"{W}val": "04A0"}
    assert outputs[0] == outputs[1]


async def test_all_flag_combinations_match_the_positive_api_and_consistent_wire_mask() -> None:
    fields = ["first_row", "last_row", "first_column", "last_column", "horizontal_banding", "vertical_banding"]
    attributes = ["firstRow", "lastRow", "firstColumn", "lastColumn", "noHBand", "noVBand"]
    for flags in itertools.product([False, True], repeat=6):
        look = d.TableLook(**dict(zip(fields, flags, strict=True)))
        data = await d.Document([d.Section([d.Table([d.TableRow([d.TableCell([d.Paragraph(text="unchanged")])])], look=look)])]).to_bytes()
        output = docx.Document(io.BytesIO(data))
        node = output.tables[0]._tbl.find(f"{W}tblPr/{W}tblLook")
        bits = [*flags[:4], not flags[4], not flags[5]]
        assert [node.get(f"{W}{name}") for name in attributes] == [str(int(flag)) for flag in bits]
        assert node.get(f"{W}val") == f"{sum(bit for flag, bit in zip(bits, [0x20, 0x40, 0x80, 0x100, 0x200, 0x400], strict=True) if flag):04X}"
        assert output.tables[0].cell(0, 0).text == "unchanged"


async def test_partial_look_override_preserves_unspecified_library_defaults() -> None:
    data = await d.Document([d.Section([d.Table([d.TableRow([d.TableCell([d.Paragraph(text="partial")])])],
        look=d.TableLook(first_row=False, last_row=True, horizontal_banding=False))])]).to_bytes()
    node = docx.Document(io.BytesIO(data)).tables[0]._tbl.find(f"{W}tblPr/{W}tblLook")
    assert node.get(f"{W}firstRow") == "0"
    assert node.get(f"{W}lastRow") == "1"
    assert node.get(f"{W}firstColumn") == "1"
    assert node.get(f"{W}noHBand") == "1"
    assert node.get(f"{W}noVBand") == "1"


async def test_deferred_look_fields_do_not_materialize_direct_conditional_formatting() -> None:
    source = d.Table([], look=lambda: d.TableLook(last_row=lambda: True, vertical_banding=lambda: True))
    hidden = d.Table([], look=lambda: (_ for _ in ()).throw(AssertionError("hidden look ran")), condition=False)
    snapshot = await d.Document([d.Section([d.Table([d.TableRow([d.TableCell([d.Paragraph(text="plain")])])],
        look=source.deferred("look"), style="Conditional"), hidden])], styles=[
        d.TableStyleDefinition(name="Conditional", last_row=d.TableSectionFormat(bold=True))]).resolve()
    data = await snapshot.to_bytes()
    assert data == await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    table = output.tables[0]
    node = table._tbl.find(f"{W}tblPr/{W}tblLook")
    assert node.get(f"{W}lastRow") == "1"
    assert node.get(f"{W}noVBand") == "0"
    assert table.cell(0, 0).paragraphs[0].runs[0].bold is None
    assert len(output.tables) == 1


async def test_template_table_look_is_not_rewritten_when_a_new_table_is_added(tmp_path: pathlib.Path) -> None:
    def table(look=None):
        return d.Table([d.TableRow([d.TableCell([d.Paragraph(text="table")])])], look=look, column_widths=[1440])
    template = tmp_path / "look-template.docx"
    template.write_bytes(await d.Document([d.Section([table(d.TableLook(first_row=False, horizontal_banding=False))])]).to_bytes())
    data = await d.Document([d.Section([table()])]).to_bytes(d.DocumentTemplate(template))
    output = docx.Document(io.BytesIO(data))
    assert len(output.tables) == 2
    existing, added = [t._tbl.find(f"{W}tblPr/{W}tblLook") for t in output.tables]
    assert existing.get(f"{W}firstRow") == "0"
    assert existing.get(f"{W}noHBand") == "1"
    assert added.get(f"{W}val") == "04A0"
    assert output.tables[0].columns[0].width == output.tables[1].columns[0].width
