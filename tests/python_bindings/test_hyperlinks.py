"""Rich external links share the native domain/codec, never a URL-fetch adapter."""
import asyncio
import io
import pathlib
import socket
import zipfile

import docx
from lxml import etree
import pytest
from rocx import declarative as d

TARGET = "https://example.invalid/λ?q=a%20b&lang=日本#résumé"
W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"


def link() -> d.Hyperlink:
    """Build formatted display runs independently of a target."""
    return d.Hyperlink(TARGET, [d.TextRun("Hello ", bold=True), d.TextRun("λ", italic=True),
                                d.Tab(), d.Break()], tooltip="A <&> λ", history=False)


async def test_links_in_all_story_contexts_preserve_targets_and_never_fetch(monkeypatch: pytest.MonkeyPatch) -> None:
    def no_network(*args: object, **kwargs: object) -> None:
        raise AssertionError("rendering must not open network connections")

    monkeypatch.setattr(socket, "create_connection", no_network)
    declaration = d.Document([d.Section(children=[d.Paragraph(children=[link(), link()]),
        d.Table(rows=[d.TableRow(children=[d.TableCell(children=[d.Paragraph(children=[link()])])])])],
        headers={"default": d.Header(children=[d.Paragraph(children=[link()])])},
        footers={"default": d.Footer(children=[d.Paragraph(children=[link()])])})])
    snapshot = await declaration.resolve()
    data = await snapshot.to_bytes()
    output = docx.Document(io.BytesIO(data))
    assert output.paragraphs[0].text == "Hello λ\t\nHello λ\t\n"
    assert output.paragraphs[0].hyperlinks[0].url == TARGET
    assert output.paragraphs[0].hyperlinks[0].runs[0].bold
    assert output.tables[0].cell(0, 0).paragraphs[0].hyperlinks[0].url == TARGET
    for owner, count in [("document", 3), ("header1", 1), ("footer1", 1)]:
        with zipfile.ZipFile(io.BytesIO(data)) as package:
            root = etree.fromstring(package.read(f"word/{owner}.xml"))
            relationships = etree.fromstring(package.read(f"word/_rels/{owner}.xml.rels"))
        links = root.findall(f".//{W}hyperlink")
        assert len(links) == count
        for node in links:
            assert node.get(f"{W}tooltip") == "A <&> λ"
            assert node.get(f"{W}history") == "0"
            relation = next(r for r in relationships if r.get("Id") == node.get(f"{R}id"))
            assert relation.get("Target") == TARGET
            assert relation.get("TargetMode") == "External"
            assert relation.get("Type") == f"{R[1:-1]}/hyperlink"
        assert len([r for r in relationships if r.get("Target") == TARGET]) == 1
    assert data == await snapshot.to_bytes()


async def test_link_fields_resolve_concurrently_and_accept_generic_references() -> None:
    arrived = 0
    ready = asyncio.Event()

    async def produce(value: object) -> object:
        nonlocal arrived
        arrived += 1
        if arrived == 4:
            ready.set()
        await asyncio.wait_for(ready.wait(), 2)
        return value

    source = d.Hyperlink(lambda: produce(TARGET), lambda: produce([d.TextRun("async λ")]),
                         tooltip=lambda: produce("async"), history=lambda: produce(True))
    consumer = d.Hyperlink(source.deferred("target"), source.deferred("children"),
                          tooltip=source.deferred("tooltip"), history=source.deferred("history"))
    data = await d.Document([d.Section(children=[d.Paragraph(children=[consumer, source])])]).to_bytes()
    links = docx.Document(io.BytesIO(data)).paragraphs[0].hyperlinks
    assert [item.text for item in links] == ["async λ", "async λ"]
    assert [item.url for item in links] == [TARGET, TARGET]
    assert arrived == 4


async def test_skipped_links_run_no_fields_and_create_no_relationships() -> None:
    def fail() -> None:
        raise AssertionError("hidden producer ran")

    hidden = d.Hyperlink(fail, fail, tooltip=fail, history=fail, condition=False)
    data = await d.Document([d.Section(children=[d.Paragraph(children=[hidden])])]).to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        assert b"/hyperlink" not in package.read("word/_rels/document.xml.rels")
    assert docx.Document(io.BytesIO(data)).paragraphs[0].hyperlinks == []


@pytest.mark.parametrize("target", ["", "#bookmark", "relative/path", "https://a b", "https://a/%GG", "https://a/%", "1bad:test"])
async def test_invalid_external_targets_have_structured_diagnostics(target: str) -> None:
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[d.Hyperlink(target, [])])])]).to_bytes()
    assert error.value.rule == "invalid_relationship"
    assert error.value.location == "target"


@pytest.mark.parametrize("child", [link(), d.Paragraph(text="block child")])
async def test_nested_links_and_block_children_are_rejected(child: object) -> None:
    with pytest.raises((ValueError, TypeError)) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[d.Hyperlink(TARGET, [child])])])]).to_bytes()
    assert error.value.rule == "invalid_content"


async def test_illegal_nested_container_is_rejected_before_native_conversion_of_its_fields() -> None:
    # The parent restriction must be checked before recursing into the inner link,
    # not after parsing arbitrary nested targets/children in the native adapter.
    nested = d.Hyperlink("relative/invalid", [])
    with pytest.raises(ValueError) as error:
        await d.Document([d.Section(children=[d.Paragraph(children=[d.Hyperlink(TARGET, [nested])])])]).to_bytes()
    assert error.value.rule == "invalid_content"
    assert error.value.location == "children[0]"


async def test_images_are_legal_display_children_and_snapshot_assets_are_captured(tmp_path: pathlib.Path) -> None:
    image = tmp_path / "image.png"
    image.write_bytes((pathlib.Path(__file__).parents[1] / "fixtures/images/sample.png").read_bytes())
    snapshot = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.Hyperlink(TARGET, [d.ImageRun(data=image), d.TextRun("picture")])])])]).resolve()
    image.unlink()
    data = await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
        assert root.find(f".//{W}hyperlink/{W}r/{W}drawing") is not None
    assert docx.Document(io.BytesIO(data)).paragraphs[0].text == "picture"
