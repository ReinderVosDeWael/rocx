"""Header and footer distances merge independently of the body margins."""
import io

import docx
import pytest

from rocx import declarative as d


async def test_header_and_footer_distances_round_trip_and_preserve_other_margins(tmp_path):
    template = tmp_path / "header-footer-distances.docx"
    initial = d.SectionProperties(page_margins={"header": 457200, "footer": 228600, "left": 685800})
    await d.Document([d.Section([], properties=initial)]).save(template)
    change = d.SectionProperties(page_margins={"header": lambda: 0})
    data = await d.Document([d.Section([], properties=change)]).to_bytes(d.DocumentTemplate(template))
    section = docx.Document(io.BytesIO(data)).sections[-1]
    assert section.header_distance == 0
    assert section.footer_distance == 228600
    assert section.left_margin == 685800


@pytest.mark.parametrize("edge", ["header", "footer"])
async def test_header_and_footer_distances_reject_negative_values(edge):
    props = d.SectionProperties(page_margins={edge: -1})
    with pytest.raises(ValueError, match=rf"page_margins\.{edge}"):
        await d.Document([d.Section([], properties=props)]).to_bytes()
