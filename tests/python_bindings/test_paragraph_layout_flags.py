"""Contextual spacing and mirrored indents are independent paragraph requests."""
import io

import docx

from rocx import declarative as d

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


async def test_paragraph_layout_flags_keep_direct_styles_and_defaults_separate():
    data = await d.Document([d.Section([
        d.Paragraph(text="omitted"),
        d.Paragraph(text="direct", style="Layout Sample", contextual_spacing=lambda: False,
                    mirror_indents=True, left_indent=12, right_indent=6),
    ])], styles=[d.ParagraphStyleDefinition("Layout Sample", contextual_spacing=True, mirror_indents=False)],
        defaults=d.DocumentDefaults(paragraph=d.ParagraphDefaults(
            contextual_spacing=False, mirror_indents=True,
        )),
    ).to_bytes()
    output = docx.Document(io.BytesIO(data))
    first, second = output.paragraphs
    for tag in ("contextualSpacing", "mirrorIndents"):
        assert first._p.find(f"{W}pPr/{W}{tag}") is None
    assert second._p.find(f"{W}pPr/{W}ind").get(f"{W}left") == "240"
    assert second._p.find(f"{W}pPr/{W}ind").get(f"{W}right") == "120"
    styles = output.styles.element
    style = next(node for node in styles if node.get(f"{W}styleId") == "LayoutSample")
    defaults = styles.find(f"{W}docDefaults/{W}pPrDefault")
    for owner, expected in [(second._p, ("0", "1")), (style, ("1", "0")), (defaults, ("0", "1"))]:
        assert tuple(owner.find(f"{W}pPr/{W}{tag}").get(f"{W}val")
                     for tag in ("contextualSpacing", "mirrorIndents")) == expected
