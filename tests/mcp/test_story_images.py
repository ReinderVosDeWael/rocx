"""Six logical header/footer slots, qualified with native shared and local previews."""

import docx
from docx.oxml.ns import qn
import pytest

from test_stdio import (
    PNG, anyio_backend, call, digest, failure, launch, oracle, parts, pytestmark, validate,
)


SLOTS = [(kind, variant) for kind in ("header", "footer")
         for variant in ("default", "first", "even")]


def slot(section, kind, variant):
    prefix = {"default": "", "first": "first_page_", "even": "even_page_"}[variant]
    return getattr(section, f"{prefix}{kind}")


def make_source(path):
    fixture = docx.Document()
    fixture.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    fixture.settings.odd_and_even_pages_header_footer = True
    fixture.add_paragraph("First section")
    first = fixture.sections[0]
    first.different_first_page_header_footer = True
    for kind, variant in SLOTS:
        slot(first, kind, variant).paragraphs[0].add_run(f"{kind} {variant}")
    fixture.add_section()
    fixture.add_paragraph("Second section")
    fixture.save(path)


@pytest.mark.parametrize(("kind", "variant"), SLOTS)
@pytest.mark.parametrize("mode", ["shared", "section-local"])
async def test_story_image_all_slots_from_paged_anchors(launch, tmp_path, kind, variant, mode):
    source = tmp_path / "source.docx"
    output = tmp_path / "edited.docx"
    make_source(source)
    expected = oracle(tmp_path / "native.docx", "story-image", source, PNG,
                      f"{kind}:{variant}", mode)

    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await call(client, "inspect_document", document=document, view={"kind": "stories"})
        rows = seen["stories"]
        selected, = [row for row in rows if row["section_index"] == 1 and
                     (row["kind"], row["variant"]) == (kind, variant)]
        assert selected["inherited"]
        staged = await call(client, "preview_edits", document=document, edits=[{
            "kind": "header-image", "anchor": selected["anchor"],
            "mode": mode, "bytes": list(PNG.read_bytes()),
        }])
        change, = staged["changes"]["header_images"]
        assert change["source_part"] == selected["owner"]["part_name"] and change["mode"] == mode
        assert change["affected_sections"] == ([0, 1] if mode == "shared" else [1])
        assert (change["output_part"] == change["source_part"]) == (mode == "shared")
        assert staged["output_snapshot"] == digest(expected)
        await call(client, "save_copy", document=document, preview=staged["preview"],
                   output=output.name)
    assert output.read_bytes() == expected
    before, after = parts(source), parts(output)
    for row in rows:
        name = row["owner"]["part_name"]
        if name != selected["owner"]["part_name"]:
            assert before[name] == after[name]
    if mode == "section-local":
        name = selected["owner"]["part_name"]
        assert before[name] == after[name]

    readback = docx.Document(output)
    for index, section in enumerate(readback.sections):
        for other_kind, other_variant in SLOTS:
            story = slot(section, other_kind, other_variant)
            has_image = bool(story._element.xpath(".//w:drawing"))
            expected_image = (other_kind, other_variant) == (kind, variant) and (
                index == 1 or mode == "shared"
            )
            assert has_image == expected_image
    validate(output)


async def test_story_image_rejects_missing_slot_and_physical_owner(launch, tmp_path):
    source = tmp_path / "source.docx"
    fixture = docx.Document()
    fixture.add_paragraph("Body")
    fixture.sections[0].footer.paragraphs[0].add_run("Footer")
    fixture.save(source)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        seen = await call(client, "inspect_document", document=document, view={"kind": "stories"})
        rows = seen["stories"]
        missing = next(row for row in rows if row["owner"] is None)
        present = next(row for row in rows if row["owner"] is not None)
        edit = {"kind": "header-image", "mode": "shared", "bytes": list(PNG.read_bytes())}
        for wrong in (missing["anchor"], present["owner"]["anchor"]):
            await failure(client, "preview_edits", "wrong_anchor_type", document=document,
                          edits=[{**edit, "anchor": wrong}])
        await call(client, "preview_edits", document=document,
                   edits=[{**edit, "anchor": present["anchor"]}])
