import asyncio
import io

import docx
import pytest
import rv_docx
from rv_docx import _native
from rv_docx import declarative as d


async def test_native_renderer_is_required_and_used(monkeypatch):
    calls = []
    original = _native.render
    def spy(declaration, template=None):
        calls.append(declaration)
        return original(declaration, template)
    monkeypatch.setattr(_native, "render", spy)
    declaration = d.Document([d.Section(children=[d.Paragraph(text="Native Rust")])])
    data = await declaration.to_bytes()
    assert _native.BACKEND == rv_docx.__backend__ == "rust"
    assert calls == [declaration]
    assert docx.Document(io.BytesIO(data)).paragraphs[0].text == "Native Rust"
    def fail(*args):
        raise RuntimeError("native renderer sentinel")
    monkeypatch.setattr(_native, "render", fail)
    with pytest.raises(RuntimeError, match="native renderer sentinel"):
        await declaration.to_docx()


async def test_returned_python_docx_object_can_be_edited_and_saved(tmp_path):
    document = await d.Document([d.Section(children=[d.Paragraph(text="Rust")])]).to_docx()
    assert isinstance(document, docx.document.Document)
    document.add_paragraph("Python edit")
    output = tmp_path / "edited.docx"
    document.save(output)
    assert [p.text for p in docx.Document(output).paragraphs] == ["Rust", "Python edit"]


async def test_all_deferred_fields_are_resolved_inplace_and_concurrently():
    started = set()
    gate = asyncio.Event()
    async def value(name, result):
        started.add(name)
        await gate.wait()
        return result
    paragraph = d.Paragraph(
        text=value("text", "resolved"),
        comment_text=value("comment", "note"),
        comment_author=value("author", "Ada"),
    )
    task = asyncio.create_task(paragraph.resolve())
    try:
        for _ in range(20):
            await asyncio.sleep(0)
            if len(started) == 3:
                break
        assert started == {"text", "comment", "author"}
    finally:
        gate.set()
    assert await task is paragraph
    assert (paragraph.text, paragraph.comment_text, paragraph.comment_author) == ("resolved", "note", "Ada")


async def test_lazy_async_children_and_header_footer_dictionaries():
    async def run():
        return d.TextRun(text=lambda: asyncio.sleep(0, result="async run"), bold=lambda: True)
    async def paragraph():
        return d.Paragraph(children=lambda: [run()])
    header = d.Header(children=lambda: [d.Paragraph(text=asyncio.sleep(0, result="header"))])
    footer = d.Footer(children=lambda: [d.Paragraph(text="footer")])
    section = d.Section(children=lambda: [paragraph()], headers={"default": header}, footers={"default": footer})
    document = await d.Document([asyncio.sleep(0, result=section)]).to_docx()
    assert document.paragraphs[0].text == "async run"
    assert document.paragraphs[0].runs[0].bold is True
    assert document.sections[0].header.paragraphs[-1].text == "header"
    assert document.sections[0].footer.paragraphs[-1].text == "footer"


async def test_conditions_skip_producers_and_preserve_identity():
    calls = []
    def forbidden():
        raise AssertionError("hidden producer was called")
    paragraph = d.Paragraph(text=lambda: calls.append("text") or "once")
    assert await paragraph is paragraph
    declaration = d.Document([d.Section(children=[paragraph,
        d.Paragraph(text=forbidden, condition=lambda: False)]),
        d.Section(children=forbidden, condition=lambda: False)])
    first = await declaration.to_docx()
    second = await declaration.to_docx()
    assert calls == ["text"]
    assert [p.text for p in first.paragraphs if p.text] == ["once"]
    assert [p.text for p in second.paragraphs if p.text] == ["once"]
    # The Python interface preserves the pinned CMI packer's final excluded section.
    assert len(first.sections) == 2


async def test_shared_components_await_the_same_producer_once():
    calls = []
    async def text():
        calls.append("called")
        await asyncio.sleep(0)
        return "shared"
    paragraph = d.Paragraph(text=text())
    document = await d.Document([d.Section(children=[paragraph, paragraph])]).to_docx()
    assert calls == ["called"]
    assert [p.text for p in document.paragraphs] == ["shared", "shared"]


async def test_producer_errors_cancel_siblings_and_propagate_original_exception():
    started = asyncio.Event()
    stopped = asyncio.Event()
    async def pending():
        try:
            started.set()
            await asyncio.Event().wait()
        finally:
            stopped.set()
    async def fail():
        await started.wait()
        raise LookupError("producer sentinel")
    paragraph = d.Paragraph(children=[d.TextRun(text=pending()), d.TextRun(text=fail())])
    with pytest.raises(LookupError, match="producer sentinel"):
        await paragraph
    assert stopped.is_set()


async def test_cyclic_component_graph_is_rejected():
    section = d.Section(children=[])
    section.children.append(section)
    with pytest.raises(ValueError, match="cannot contain itself"):
        await section


async def test_save_supports_paths_and_binary_streams(tmp_path):
    declaration = d.Document([d.Section(children=[d.Paragraph(text="saved")])])
    output = tmp_path / "output.docx"
    await declaration.save(output)
    assert docx.Document(output).paragraphs[0].text == "saved"
    stream = io.BytesIO()
    await declaration.save(stream)
    assert docx.Document(io.BytesIO(stream.getvalue())).paragraphs[0].text == "saved"


async def test_native_io_errors_keep_python_exception_types(tmp_path):
    declaration = d.Document([d.Section(children=[d.Paragraph(children=[d.ImageRun(data=tmp_path / "missing.png")])])])
    with pytest.raises(FileNotFoundError):
        await declaration.to_docx()


async def test_false_image_condition_never_reads_path(tmp_path):
    document = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.ImageRun(data=tmp_path / "missing.png", condition=lambda: False),
        d.TextRun(text="visible"),
    ])])]).to_docx()
    assert document.paragraphs[0].text == "visible"


async def test_subclasses_keep_parent_component_semantics():
    class CustomParagraph(d.Paragraph):
        pass
    document = await d.Document([d.Section(children=[CustomParagraph(text="subclass")])]).to_docx()
    assert document.paragraphs[0].text == "subclass"
