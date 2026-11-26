"""Direct document defaults and package properties through installed MCP."""

import subprocess

import docx
from docx.opc.constants import RELATIONSHIP_TYPE as RT
from docx.opc.packuri import PackURI
from docx.opc.part import Part
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from jsonschema.validators import validator_for
from lxml import etree
import pytest

from test_stdio import anyio_backend, call, digest, executable, failure, launch, parts, validate

pytestmark = pytest.mark.anyio

STYLES = "word/resources/defaults.xml"
SETTINGS = "word/resources/settings.xml"
CORE = "docProps/core.xml"
EXTENDED = "docProps/app.xml"
CUSTOM = "docProps/resources/custom.xml"
EXT = "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
CP = "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties"
VT = "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"


def make_source(path, *, duplicate_description=False):
    document = docx.Document()
    document.add_paragraph("Body text")
    document.sections[0].header.paragraphs[0].text = "Keep this header"
    core = document.core_properties
    core.title = "Initial title"
    core.subject = "Initial subject"
    core.comments = "Initial description"
    if duplicate_description:
        duplicate = OxmlElement("dc:description")
        duplicate.text = "Duplicate description"
        core._element.append(duplicate)

    settings = document.part.part_related_by(RT.SETTINGS)
    settings.partname = PackURI(f"/{SETTINGS}")
    settings.element.find(qn("w:zoom")).set(qn("w:percent"), "100")

    styles = document.part.part_related_by(RT.STYLES)
    styles.partname = PackURI(f"/{STYLES}")
    defaults = styles.element.find(qn("w:docDefaults"))
    defaults.find(f"{qn('w:rPrDefault')}/{qn('w:rPr')}").append(OxmlElement("w:b"))
    paragraph = defaults.find(f"{qn('w:pPrDefault')}/{qn('w:pPr')}")
    alignment = OxmlElement("w:jc")
    alignment.set(qn("w:val"), "center")
    paragraph.append(alignment)
    paragraph.find(qn("w:spacing")).set(qn("w:after"), "00200")

    package = document.part.package
    extended = package.part_related_by(RT.EXTENDED_PROPERTIES)
    props = etree.fromstring(extended.blob)
    props.find(f"{{{EXT}}}Company").text = "Remove Company"
    extended._blob = etree.tostring(props)

    custom = etree.Element(f"{{{CP}}}Properties", nsmap={None: CP, "vt": VT})
    for pid, name, kind, value in (
        (7, "Reviewed", "lpwstr", "pending"),
        (8, "LegacyNumber", "i4", "42"),
        (9, "Disabled", "bool", "false"),
        (10, "Blank", "lpwstr", ""),
    ):
        prop = etree.SubElement(
            custom, f"{{{CP}}}property", name=name, pid=str(pid),
            fmtid="{D5CDD505-2E9C-101B-9397-08002B2CF9AE}",
        )
        etree.SubElement(prop, f"{{{VT}}}{kind}").text = value
    package.relate_to(
        Part(
            PackURI(f"/{CUSTOM}"),
            "application/vnd.openxmlformats-officedocument.custom-properties+xml",
            etree.tostring(custom),
        ),
        RT.CUSTOM_PROPERTIES,
    )
    document.save(path)
    return path.read_bytes()


def resource(kind, **members):
    return dict(kind="resource", edit=dict(kind=kind, **members))


async def read_resources(client, document, **selector):
    return await call(client, "inspect_document", document=document,
                      view=dict(kind="resources", **selector))


def resource_query(kind, **selector):
    return dict(kind="resource-value", query=dict(kind=kind, **selector))


def oracle(output, source, case):
    binary = executable("ROCX_MCP_ORACLE").with_name("mcp_resource_oracle")
    subprocess.run([binary, output, source, case], check=True, cwd=output.parent, timeout=20)
    return output.read_bytes()


TABS = [
    dict(position_twips=480, alignment="left"),
    dict(position_twips=960, alignment="decimal", leader="dot"),
]
MIXED = [
    resource("core-title", value="MCP & Rust"),
    resource("core-subject", value=None),
    resource("default-alignment", value="justify"),
    resource("default", field="bold", value=False),
    resource("default", field="beforeTwips", value=288),
    resource("default", field="lineRule", value="atLeast"),
    resource("default", field="tabStops", value=TABS),
    resource("default", field="trackRevisions", value=False),
    resource("metadata", field="comments", value="Review 😀"),
    resource("metadata", field="company", value=None),
    resource("custom-property", name="Reviewed", value=False),
]
DIRECT = [
    resource("default", field="bold", value=None),
    resource("default", field="justification", value="start"),
    resource("default", field="beforeTwips", value=4294967295),
    resource("default", field="leftTwips", value=-2147483648),
    resource("custom-property", name="Reviewed", value=None),
    resource("custom-property", name="Empty", value=""),
]
MALFORMED = [
    resource("core-title"),
    resource("core-subject", value=None, extra=True),
    resource("default-alignment", value="both"),
    resource("default", field="unknown", value=True),
    resource("default", field="beforeTwips", value=4294967296),
    resource("default", field="leftTwips", value=-2147483649),
    resource("default", field="tabStops", value=[dict(TABS[0], extra=True)]),
    resource("metadata", field="unknown", value="value"),
    resource("custom-property", name="Reviewed", value=1),
    dict(MIXED[0], anchor="unexpected"),
]


async def test_resource_discovery(launch):
    async with launch() as client:
        tools = {tool.name: tool.inputSchema for tool in (await client.list_tools()).tools}
    preview = validator_for(tools["preview_edits"])(tools["preview_edits"])
    for edit in [*MIXED, *DIRECT]:
        assert preview.is_valid(dict(document="doc", edits=[edit])), edit
        missing = resource(
            edit["edit"]["kind"],
            **{k: v for k, v in edit["edit"].items() if k not in {"kind", "value"}},
        )
        assert not preview.is_valid(dict(document="doc", edits=[missing])), missing
    for edit in MALFORMED:
        assert not preview.is_valid(dict(document="doc", edits=[edit])), edit

    inspect = validator_for(tools["inspect_document"])(tools["inspect_document"])
    for view in [dict(kind="resources"), dict(kind="resources", custom_property="Reviewed")]:
        assert inspect.is_valid(dict(document="doc", view=view))
    for view in [dict(kind="resources", custom_property=None), dict(kind="resources", extra=True)]:
        assert not inspect.is_valid(dict(document="doc", view=view))
    for query in [
        resource_query("document-defaults"),
        resource_query("default-alignment"),
        resource_query("metadata", field="comments"),
        resource_query("custom-property", name="Reviewed"),
    ]:
        assert inspect.is_valid(dict(document="doc", view=query)), query
    for query in [
        dict(kind="resource-value"),
        resource_query("document-defaults", extra=True),
        resource_query("default-alignment", field="justification"),
        resource_query("metadata"),
        resource_query("metadata", field="nonexistent"),
        resource_query("custom-property", name=None),
        resource_query("custom-property"),
    ]:
        assert not inspect.is_valid(dict(document="doc", view=query)), query


async def test_resource_value_queries_isolate_malformed_metadata(launch, tmp_path):
    source = tmp_path / "duplicate.docx"
    make_source(source, duplicate_description=True)
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="resources"))
        for query in [
            resource_query("metadata", field="description"),
            resource_query("metadata", field="comments"),
            resource_query("custom-property", name="LegacyNumber"),
        ]:
            await failure(client, "inspect_document", "inspection_failed", document=document,
                          view=query)
        await failure(client, "inspect_document", "invalid_input", document=document,
                      view=resource_query("default-alignment", extra=True))

        async def value(kind, **selector):
            result = await call(client, "inspect_document", document=document,
                                view=resource_query(kind, **selector))
            assert result["document"] == document
            return result["value"]

        # Title shares the malformed description's physical part.
        assert await value("metadata", field="title") == "Initial title"
        assert await value("metadata", field="identifier") is None
        assert await value("custom-property", name="Reviewed") == "pending"
        assert await value("custom-property", name="Disabled") is False
        assert await value("custom-property", name="Blank") == ""
        assert await value("custom-property", name="blank") is None
        assert await value("default-alignment") == "center"
        assert (await value("document-defaults"))["bold"] is True


async def test_mixed_resource_batch_native_bytes_and_preservation(launch, tmp_path):
    source = tmp_path / "source.docx"
    original = make_source(source)
    output = tmp_path / "mixed.docx"
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        initial = await read_resources(client, document)
        assert initial["default_alignment"] == "center"
        assert initial["document_defaults"]["bold"] is True
        assert initial["document_defaults"]["afterTwips"] == 200
        assert "trackRevisions" not in initial["document_defaults"]
        assert "custom_property" not in initial
        assert initial["metadata"]["description"] == initial["metadata"]["comments"] == (
            "Initial description"
        )
        assert (await read_resources(client, document, custom_property="Reviewed"))[
            "custom_property"
        ] == "pending"
        assert (await read_resources(client, document, custom_property="reviewed"))[
            "custom_property"
        ] is None

        body = await call(client, "inspect_document", document=document)
        insertion = dict(kind="insert-text", anchor=body["body"][0]["anchor"],
                         offset=4, text=" & added")
        preview = await call(client, "preview_edits", document=document, edits=[*MIXED, insertion])
        saved = await call(client, "save_copy", document=document, preview=preview["preview"],
                           output=output.name)
        assert await read_resources(client, document) == initial

        reopened = (await call(client, "open_document", path=output.name))["document"]
        readback = await read_resources(client, reopened, custom_property="Reviewed")
        assert readback["default_alignment"] == "justify"
        assert readback["custom_property"] is False
        defaults = readback["document_defaults"]
        assert defaults["bold"] is False
        assert defaults["trackRevisions"] is False
        assert defaults.items() >= {
            "beforeTwips": 288, "afterTwips": 200, "justification": "both", "lineRule": "atLeast",
        }.items()
        assert defaults["tabStops"] == [dict(TABS[0], leader=None), TABS[1]]
        assert readback["metadata"].items() >= {
            "title": "MCP & Rust", "subject": None, "company": None, "description": "Review 😀",
            "comments": "Review 😀",
        }.items()

    assert source.read_bytes() == original
    assert output.read_bytes() == oracle(tmp_path / "native.docx", source, "mixed")
    assert saved["output_snapshot"] == preview["output_snapshot"] == digest(output.read_bytes())
    reopened = docx.Document(output)
    assert reopened.paragraphs[0].text == "Body & added text"
    assert reopened.core_properties.title == "MCP & Rust"
    assert reopened.core_properties.comments == "Review 😀"
    assert reopened.core_properties.subject == ""

    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {k for k in before if before[k] != after[k]} == {
        STYLES, SETTINGS, CORE, EXTENDED, CUSTOM, "word/document.xml",
    }
    styles = etree.fromstring(after[STYLES])
    spacing_path = "/".join(map(qn, ("w:docDefaults", "w:pPrDefault", "w:pPr", "w:spacing")))
    spacing = styles.find(spacing_path)
    assert spacing.get(qn("w:after")) == "00200"
    custom_before, custom_after = (etree.fromstring(package[CUSTOM]) for package in (before, after))
    assert custom_before[0].attrib == custom_after[0].attrib
    assert custom_after[0][0].tag == f"{{{VT}}}bool"
    assert custom_after[0][0].text == "false"
    assert etree.tostring(custom_before[1], method="c14n", exclusive=True) == etree.tostring(
        custom_after[1], method="c14n", exclusive=True,
    )
    validate(source, output)


async def test_resource_only_rejection_and_retry_without_inspection(launch, tmp_path):
    source = tmp_path / "source.docx"
    original = make_source(source)
    output = tmp_path / "direct.docx"
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        for edit in MALFORMED:
            await failure(client, "preview_edits", "invalid_input", document=document, edits=[edit])
        for edits in [
            [MIXED[0], resource("metadata", field="title", value=None)],
            [MIXED[1], resource("metadata", field="subject", value=None)],
            [MIXED[2], DIRECT[1]],
            [MIXED[-3], resource("metadata", field="description", value=None)],
            [DIRECT[-2], MIXED[-1]],
            [resource("default", field="bold", value="false")],
            [resource("custom-property", name="LegacyNumber", value=None)],
        ]:
            await failure(client, "preview_edits", "rejected_batch", document=document, edits=edits)
        assert list(tmp_path.iterdir()) == [source]
        preview = await call(client, "preview_edits", document=document, edits=DIRECT)
        await call(client, "save_copy", document=document, preview=preview["preview"],
                   output=output.name)

        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="resources", custom_property="LegacyNumber"))
        await failure(client, "inspect_document", "invalid_input", document=document,
                      view=dict(kind="resources", custom_property=None))
        reopened = (await call(client, "open_document", path=output.name))["document"]
        readback = await read_resources(client, reopened, custom_property="Reviewed")
        assert readback["default_alignment"] is None
        assert readback["default_alignment_error"]
        defaults = readback["document_defaults"]
        assert "bold" not in defaults
        assert defaults["justification"] == "start"
        assert defaults["beforeTwips"] == 4294967295
        assert defaults["leftTwips"] == -2147483648
        assert readback["custom_property"] is None
        assert (await read_resources(client, reopened, custom_property="Empty"))[
            "custom_property"
        ] == ""

    assert source.read_bytes() == original
    assert output.read_bytes() == oracle(tmp_path / "native.docx", source, "direct")
    before, after = parts(source), parts(output)
    assert before.keys() == after.keys()
    assert {k for k in before if before[k] != after[k]} == {STYLES, CUSTOM}
