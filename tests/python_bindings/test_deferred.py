"""Property dependencies use actual resolved values and native DOCX output."""
import asyncio

import docx
import pytest
from rv_docx import declarative as d


async def render(*children):
    return await d.Document([d.Section(children=list(children))]).to_docx()


@pytest.mark.parametrize("source_first", [True, False])
async def test_paragraph_text_dependency_is_order_independent_and_saved(source_first, tmp_path):
    calls = []

    async def fetch():
        calls.append("source")
        await asyncio.sleep(0)
        return "42 λ"

    source = d.Paragraph(text=fetch())
    result = source.deferred("text")

    async def summarize():
        return f"Result: {await result}"

    consumer = d.Paragraph(text=summarize())
    children = [source, consumer] if source_first else [consumer, source]
    declaration = d.Document([d.Section(children=children)])
    output = tmp_path / "dependencies.docx"
    await declaration.save(output)
    expected = ["42 λ", "Result: 42 λ"] if source_first else ["Result: 42 λ", "42 λ"]
    assert [p.text for p in docx.Document(output).paragraphs] == expected
    assert source.text == "42 λ"
    assert consumer.text == "Result: 42 λ"
    assert calls == ["source"]
    assert [p.text for p in (await declaration.to_docx()).paragraphs] == expected
    assert calls == ["source"]


async def test_fanout_and_multistage_dependencies_share_one_producer():
    calls = []

    async def fetch():
        calls.append(1)
        await asyncio.sleep(0)
        return "seed"

    source = d.Paragraph(text=fetch())
    left = d.Paragraph(text=source.deferred("text"))
    right = d.Paragraph(text=source.deferred("text"))

    async def combine():
        a, b = await asyncio.gather(left.deferred("text"), right.deferred("text"))
        return f"{a}/{b}"

    end = d.Paragraph(text=combine())
    document = await render(end, right, left, source)
    assert [p.text for p in document.paragraphs] == ["seed/seed", "seed", "seed", "seed"]
    assert calls == [1]


async def test_reciprocal_components_with_acyclic_fields_do_not_deadlock():
    left = d.Paragraph(text="left", comment_author=asyncio.sleep(0, result="Ada"))
    right = d.Paragraph(text=left.deferred("text"), comment_author=asyncio.sleep(0, result="Grace"))
    left.comment_text = right.deferred("text")
    right.comment_text = left.deferred("comment_author")
    document = await asyncio.wait_for(render(right, left), timeout=5)
    assert [p.text for p in document.paragraphs] == ["left", "left"]
    assert [(c.text, c.author) for c in document.comments] == [("Ada", "Grace"), ("left", "Ada")]


async def test_same_component_can_depend_on_an_independent_property():
    paragraph = d.Paragraph(text="unused", comment_text=asyncio.sleep(0, result="note"))

    async def text():
        return f"Text about {await paragraph.deferred('comment_text')}"

    paragraph.text = text()
    document = await asyncio.wait_for(render(paragraph), timeout=5)
    assert document.paragraphs[0].text == "Text about note"
    assert next(iter(document.comments)).text == "note"


async def test_dependencies_cover_formatting_comments_nested_children_and_metadata():
    run = d.TextRun(text=asyncio.sleep(0, result="origin"),
                    bold=asyncio.sleep(0, result=True),
                    size=asyncio.sleep(0, result=18),
                    color=asyncio.sleep(0, result=(12, 34, 56)),
                    comment_text=asyncio.sleep(0, result="shared comment"),
                    comment_author=asyncio.sleep(0, result="Ada"))
    copy = d.TextRun(text=run.deferred("text"), bold=run.deferred("bold"),
                     size=run.deferred("size"), color=run.deferred("color"),
                     comment_text=run.deferred("comment_text"),
                     comment_author=run.deferred("comment_author"))
    paragraph = d.Paragraph(children=lambda: asyncio.sleep(0, result=[copy]))
    section = d.Section(children=[d.Paragraph(children=[run]), paragraph])
    declaration = d.Document([section], title=run.deferred("text"))
    declaration.creator = declaration.deferred("title")
    document = await declaration.to_docx()
    assert document.core_properties.title == document.core_properties.author == "origin"
    assert [p.text for p in document.paragraphs] == ["origin", "origin"]
    for p in document.paragraphs:
        assert p.runs[0].bold is True
        assert p.runs[0].font.size.pt == 18
        assert str(p.runs[0].font.color.rgb) == "0C2238"
    assert [(c.text, c.author) for c in document.comments] == [("shared comment", "Ada")] * 2


async def test_children_properties_can_be_shared_across_component_types():
    calls = []

    async def content():
        calls.append(1)
        return [d.Paragraph(text="dependent cell and header")]

    header = d.Header(children=content())
    cell = d.TableCell(children=header.deferred("children"))
    section = d.Section(children=[d.Table(rows=[d.TableRow(children=[cell])])], headers={"default": header})
    document = await d.Document([section]).to_docx()
    assert document.tables[0].cell(0, 0).text == "dependent cell and header"
    assert document.sections[0].header.paragraphs[-1].text == "dependent cell and header"
    assert calls == [1]


async def test_plain_dataclass_properties_and_image_data_are_supported():
    from pathlib import Path
    image = d.ImageRun(data=asyncio.sleep(0, result=Path(__file__).parents[1] / "fixtures" / "images" / "sample.png"))
    image_copy = d.ImageRun(data=image.deferred("data"), transformation=image.deferred("transformation"))
    properties = d.SectionProperties(page_margins=lambda: asyncio.sleep(0, result={"left": 914400}))
    image.transformation = lambda: asyncio.sleep(0, result={"width": 32, "height": 16})
    other = d.SectionProperties(page_margins=d.DeferredProperty(properties, "page_margins"))
    section = d.Section(children=[d.Paragraph(children=[image_copy, image])], properties=other)
    document = await d.Document([section]).to_docx()
    assert document.sections[0].left_margin == 914400
    assert [(shape.width, shape.height) for shape in document.inline_shapes] == [(406400, 203200)] * 2


async def test_property_only_dependency_does_not_run_unrelated_fields():
    def forbidden():
        raise AssertionError("unrelated field was evaluated")

    source = d.Paragraph(text=lambda: asyncio.sleep(0, result="needed"), comment_text=forbidden)
    document = await render(d.Paragraph(text=source.deferred("text")))
    assert document.paragraphs[0].text == "needed"
    assert callable(source.comment_text)
    assert await source.deferred("text") == "needed"


@pytest.mark.parametrize("length", [1, 2, 3])
async def test_dependency_cycles_fail_instead_of_waiting_forever(length):
    paragraphs = [d.Paragraph(text="placeholder") for _ in range(length)]
    for i, paragraph in enumerate(paragraphs):
        paragraph.text = paragraphs[(i + 1) % length].deferred("text")
    with pytest.raises(ValueError, match=r"Cyclic deferred property dependency: Paragraph.text"):
        await asyncio.wait_for(render(*paragraphs), timeout=5)


async def test_condition_is_shared_and_excluded_dependency_does_not_run():
    calls = []
    source = d.Paragraph(text=lambda: calls.append("producer") or "hidden",
                         condition=lambda: calls.append("condition") or False)
    with pytest.raises(ValueError, match="excluded Paragraph"):
        await asyncio.wait_for(render(d.Paragraph(text=source.deferred("text")), source), timeout=5)
    assert calls == ["condition"]


async def test_hidden_consumer_does_not_evaluate_dependency():
    calls = []
    source = d.Paragraph(text=lambda: calls.append(1) or "hidden")
    document = await render(d.Paragraph(text=source.deferred("text"), condition=False))
    assert document.paragraphs == []
    assert calls == []


async def test_dependency_error_is_original_and_cancels_other_work():
    failure = LookupError("dependency failed")
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
        raise failure

    source = d.Paragraph(text=fail())
    with pytest.raises(LookupError) as error:
        await render(d.Paragraph(text=source.deferred("text")), d.Paragraph(text=pending()), source)
    assert error.value is failure
    assert stopped.is_set()


async def test_successful_standalone_reference_is_reusable():
    calls = []
    source = d.Paragraph(text=lambda: calls.append(1) or asyncio.sleep(0, result="once"))
    reference = d.DeferredProperty[str](source, "text")
    assert calls == []
    assert await reference == await reference == "once"
    assert calls == [1]


@pytest.mark.parametrize("name", ["missing", "condition", "resolve", "_active"])
def test_invalid_properties_are_rejected_without_evaluation(name):
    source = d.Paragraph(text=lambda: pytest.fail("producer was evaluated"))
    with pytest.raises(AttributeError, match="no resolvable property"):
        source.deferred(name)
