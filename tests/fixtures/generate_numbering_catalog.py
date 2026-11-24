"""Build a python-docx package with independent numbering-style edges."""
from copy import deepcopy
from io import BytesIO
from zipfile import ZipFile

from docx import Document
from docx.enum.style import WD_STYLE_TYPE
from docx.oxml.ns import qn
from lxml import etree


def source(fault=None):
    doc = Document()
    doc.settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")
    doc.add_paragraph("List through a numbering style")
    style = doc.styles.add_style("Fixture List", WD_STYLE_TYPE.LIST)

    def append(parent, tag, **attrs):
        return etree.SubElement(parent, qn(f"w:{tag}"), {qn(f"w:{k}"): str(v) for k, v in attrs.items()})

    props = append(style.element, "pPr")
    append(append(props, "numPr"), "numId", val=713 if fault == "nonreciprocal" else 712)
    root = doc.part.numbering_part.element
    owner = append(root, "abstractNum", abstractNumId=710)
    alias = append(root, "abstractNum", abstractNumId=711)
    append(owner, "styleLink", val="MissingStyle" if fault == "dangling" else style.style_id)
    append(alias, "numStyleLink", val=style.style_id)
    first_num = root.find(qn("w:num"))
    first_num.addprevious(owner)
    first_num.addprevious(alias)
    level = append(owner, "lvl", ilvl=0)
    append(level, "start", val="+01")
    append(level, "numFmt", val="lowerRoman")
    append(level, "lvlRestart", val=0)
    append(level, "pStyle", val="Heading1")
    append(level, "isLgl", val=0)
    append(level, "suff", val="nothing")
    append(level, "lvlText", val="(%1)")
    append(level, "lvlJc", val="end")
    p = append(level, "pPr")
    append(p, "keepNext", val=0)
    append(append(p, "tabs"), "tab", val="right", pos="240", leader="dot")
    append(p, "spacing", before=20, after=40, line=200, lineRule="exact")
    append(p, "ind", left=480, right=40, hanging=240)
    append(p, "jc", val="center")
    r = append(level, "rPr")
    append(r, "rFonts", ascii="Aptos", hAnsi="Aptos", eastAsia="Mincho", cs="Arial")
    append(r, "b", val=0)
    append(r, "i", val=1)
    append(r, "bCs", val=0)
    append(r, "color", val="auto", themeColor="accent2", themeTint="10")
    append(r, "sz", val=25)
    append(r, "szCs", val=26)
    append(r, "u", val="double")
    append(r, "vertAlign", val="subscript")
    append(r, "lang", val="en-US", eastAsia="ja-JP", bidi="ar")
    for num, abstract in [(712, 710), (713, 711)]:
        instance = append(root, "num", numId=num)
        append(instance, "abstractNumId", val=abstract)
        if num == 712:
            override = append(instance, "lvlOverride", ilvl=0)
            append(override, "startOverride", val=6)
            override.append(deepcopy(level))
    if fault == "opaque":
        # Well-formed lexical content not expressible in a level declaration.
        r.append(etree.Comment("retain inside the label format"))
    stream = BytesIO()
    doc.save(stream)
    return stream.getvalue()


def parts(data):
    with ZipFile(BytesIO(data)) as archive:
        return {name: archive.read(name) for name in archive.namelist()}


LEVEL = dict(level=0, start=5, format="ordinal", text="%1)", restart=0,
             is_legal=False, suffix="space", alignment="start", paragraph_style="Heading 2",
             label_format=dict(bold=False, size=12.5, underline="double",
                               color=dict(theme="accent2", tint=16)),
             paragraph_format=dict(alignment="center", first_line_indent=-9,
                                   line_height=dict(kind="exact", value=11),
                                   tab_stops=[dict(position=12, alignment="right", leader="dot")]))


def edits(doc):
    from rocx import declarative as d
    from rocx.editor import ResourceEdit as R

    p = LEVEL["paragraph_format"]
    level = d.NumberingLevel(**(LEVEL | {
        "label_format": d.RunDefaults(**LEVEL["label_format"]),
        "paragraph_format": d.ParagraphDefaults(**(p | {
            "line_height": d.LineHeight.exact(p["line_height"]["value"]),
            "tab_stops": [d.TabStop(**t) for t in p["tab_stops"]],
        })),
    }))
    return [R.replace_abstract(710, [level]), R.paragraph_numbering(doc.body[0].anchor, (713, 0)),
            R.document_default("trackRevisions", False), R.metadata("comments", "Numbering fixture review")]
