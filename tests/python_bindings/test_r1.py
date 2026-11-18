"""Cross-language R1 metadata/children, domain diagnostics and render policy."""
import io
import pathlib
import zipfile

import docx
import pytest
import rv_docx
from rv_docx import declarative as d


async def test_metadata_and_section_table_cell_fields_share_generic_references() -> None:
    calls = []
    source = d.Document([], title=lambda: calls.append("title") or "R1 λ")
    table_source = d.Table(rows=[], column_widths=lambda: [1440])
    cell_source = d.TableCell(background_color=lambda: (12, 34, 56))
    cell = d.TableCell(children=[d.Paragraph(text="cell")],
                       background_color=cell_source.deferred("background_color"))
    table = d.Table(rows=[d.TableRow(children=[cell])],
                    column_widths=table_source.deferred("column_widths"))
    properties = d.SectionProperties(columns=lambda: {"count": 2})
    document = d.Document([d.Section(children=[d.Paragraph(text=source.deferred("title")), table],
                                    properties=properties)], title=source.deferred("title"))
    snapshot = await document.resolve()
    first = await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(first))
    assert output.core_properties.title == output.paragraphs[0].text == "R1 λ"
    namespace = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
    shading = output.tables[0].cell(0, 0)._tc.tcPr.find(f"{namespace}shd")
    assert shading is not None
    assert shading.get(f"{namespace}fill") == "0C2238"
    assert calls == ["title"]
    assert first == await snapshot.to_bytes()


@pytest.mark.parametrize("field,value,rule", [
    ("size", -1, "invalid_measurement"),
    ("size", float("nan"), "invalid_measurement"),
    ("size", float("inf"), "invalid_measurement"),
    ("size", 1e100, "invalid_measurement"),
])
async def test_native_domain_failures_expose_rule_location_and_cause(field: str, value: float, rule: str) -> None:
    run = d.TextRun(text="invalid")
    setattr(run, field, value)
    document = d.Document([d.Section(children=[d.Paragraph(children=[run])])])
    with pytest.raises(ValueError) as error:
        await document.to_bytes()
    assert error.value.rule == rule
    assert error.value.location == f"sections[0].children[0].children[0].format.{field}"
    assert hasattr(error.value, "cause")


async def test_producer_and_dependency_errors_keep_categories_and_machine_context() -> None:
    cause = LookupError("original cause")
    failure = PermissionError("producer sentinel")

    def fail() -> None:
        raise failure from cause

    with pytest.raises(PermissionError) as error:
        await d.Paragraph(text=fail).resolve()
    assert error.value is failure
    assert error.value.__cause__ is cause
    assert error.value.rule == "producer_error"
    assert error.value.location == "Paragraph.text"
    assert error.value.cause == "original cause"

    hidden = d.Paragraph(text="hidden", condition=False)
    with pytest.raises(ValueError) as error:
        await hidden.deferred("text")
    assert error.value.rule == "excluded_dependency"
    assert error.value.location == "Paragraph.text"


async def test_producer_exceptions_with_readonly_attribute_setters_are_not_replaced() -> None:
    class FrozenError(RuntimeError):
        def __setattr__(self, name: str, value: object) -> None:
            raise AttributeError("readonly producer exception")

    failure = FrozenError("original")

    def fail() -> None:
        raise failure

    with pytest.raises(FrozenError) as error:
        await d.Paragraph(text=fail).resolve()
    assert error.value is failure
    assert error.value.rule == "producer_error"


async def test_render_options_are_explicit_and_do_not_change_snapshots(tmp_path: pathlib.Path) -> None:
    declaration = d.Document([d.Section(children=[d.Paragraph(text="options")])],
                              timestamp="2026-11-18T00:00:00Z")
    snapshot = await declaration.resolve()
    original = await snapshot.to_bytes()
    assert original == await snapshot.to_bytes(options=rv_docx.RenderOptions())
    options = rv_docx.RenderOptions(timestamp="2026-01-01T00:00:00Z")
    overridden = await snapshot.to_bytes(options=options)
    with zipfile.ZipFile(io.BytesIO(overridden)) as package:
        assert b"2026-01-01T00:00:00Z" in package.read("docProps/core.xml")
    assert original == await snapshot.to_bytes()
    assert snapshot.timestamp == "2026-11-18T00:00:00Z"
    output = tmp_path / "options.docx"
    await snapshot.save(output, options=options)
    assert output.read_bytes() == overridden
    assert (await snapshot.to_docx(options=options)).paragraphs[0].text == "options"
    assert await declaration.to_bytes(options=options) == overridden
    with pytest.raises(ValueError) as error:
        await snapshot.to_bytes(options=rv_docx.RenderOptions(profile="strict"))
    assert error.value.rule == "unsupported_profile"
    assert error.value.location == "options.profile"
    with pytest.raises(ValueError) as error:
        await snapshot.to_bytes(options=rv_docx.RenderOptions(read_limits=rv_docx.ReadLimits(max_parts=1)))
    assert error.value.rule == "invalid_package"
    assert error.value.location == "package"
    template = tmp_path / "template.docx"
    template.write_bytes(original)
    with pytest.raises(ValueError) as error:
        await snapshot.to_bytes(d.DocumentTemplate(template), options=rv_docx.RenderOptions(read_limits=rv_docx.ReadLimits(max_parts=1)))
    assert error.value.rule == "invalid_package"


@pytest.mark.parametrize("name", ["max_parts", "max_part_bytes", "max_expanded_bytes"])
async def test_all_template_read_budgets_are_enforced(name: str) -> None:
    snapshot = await d.Document([]).resolve()
    options = rv_docx.RenderOptions(read_limits=rv_docx.ReadLimits(**{name: 1}))
    with pytest.raises(ValueError) as error:
        await snapshot.to_bytes(options=options)
    assert error.value.rule == "invalid_package"
    assert error.value.location is not None


async def test_native_conversion_and_io_errors_keep_their_exception_categories(tmp_path: pathlib.Path) -> None:
    document = d.Document([d.Section(children=[d.Paragraph(text="native")])])
    with pytest.raises(FileNotFoundError) as error:
        await document.to_bytes(d.DocumentTemplate(tmp_path / "missing.docx"))
    assert error.value.rule == "io_error"
    assert error.value.cause
    assert error.value.__cause__ is not None
    with pytest.raises((ValueError, TypeError)) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[d.Break(type="bad")])])]).to_bytes()
    assert error.value.rule == "invalid_content"
    assert error.value.location == "document"

    with pytest.raises(FileNotFoundError) as error:
        await document.save(tmp_path / "missing-directory/output.docx")
    assert error.value.rule == "io_error"
    assert error.value.location == "output"
    with pytest.raises(FileNotFoundError) as error:
        await d.ImageRun(data=tmp_path / "missing-image.png").resolve()
    assert error.value.rule == "io_error"
    assert error.value.location == "ImageRun.data"
    with pytest.raises(KeyError) as error:
        await d.Document([d.Section(children=[d.Paragraph(text="unknown", style="Absent R1 style")])]).to_bytes()
    assert error.value.rule == "unknown_style"
    assert error.value.location == "/word/document.xml"
