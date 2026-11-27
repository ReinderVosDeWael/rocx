"""Direct document defaults and package properties through installed MCP."""

import json
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

from test_stdio import (
    anyio_backend, assert_tool_failure, call, digest, executable, failure, launch, parts, validate,
)

pytestmark = pytest.mark.anyio

STYLES = "word/resources/defaults.xml"
SETTINGS = "word/resources/settings.xml"
CORE = "docProps/core.xml"
EXTENDED = "docProps/app.xml"
CUSTOM = "docProps/resources/custom.xml"
EXT = "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
CP = "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties"
VT = "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"


def word_defaults(document):
    defaults = document.part.part_related_by(RT.STYLES).element.find(qn("w:docDefaults"))
    run = defaults.find(f"{qn('w:rPrDefault')}/{qn('w:rPr')}")
    paragraph = defaults.find(f"{qn('w:pPrDefault')}/{qn('w:pPr')}")
    return run, paragraph


def build_source(*, duplicate_description=False):
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
    run, paragraph = word_defaults(document)
    run.append(OxmlElement("w:b"))
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
    return document


def make_source(path, **kwargs):
    document = build_source(**kwargs)
    document.save(path)
    return path.read_bytes()


def resource(kind, **members):
    return dict(kind="resource", edit=dict(kind=kind, **members))


async def read_resources(client, document, **selector):
    return await call(client, "inspect_document", document=document,
                      view=dict(kind="resources", **selector))


def resource_query(kind, **selector):
    return dict(kind="resource-value", query=dict(kind=kind, **selector))


def resource_oracle(*args, cwd):
    binary = executable("ROCX_MCP_ORACLE").with_name("mcp_resource_oracle")
    return subprocess.run([binary, *args], check=True, cwd=cwd, timeout=20, stdout=subprocess.PIPE)


def oracle(output, source, case):
    resource_oracle(output, source, case, cwd=output.parent)
    return output.read_bytes()


def native_read(source, kind, *fields):
    return json.loads(resource_oracle("inspect", source, kind, *fields, cwd=source.parent).stdout)


async def assert_readback(client, document, query, native, expected, *, error=False):
    arguments = dict(document=document, view=query)
    if error:
        assert expected in native["error"], query
        result = await client.call_tool("inspect_document", arguments)
        assert_tool_failure(result, "inspection_failed")
        assert result.structuredContent["message"] == native["error"], query
    else:
        result = await call(client, "inspect_document", **arguments)
        assert result["document"] == document
        # JSON equality preserves the distinction between numbers and booleans.
        assert json.dumps(result["value"], sort_keys=True) == json.dumps(
            native["value"], sort_keys=True,
        ) == json.dumps(expected, sort_keys=True), query


CORE_FIELDS = [
    ("title", "dc:title", "A title & subject"),
    ("subject", "dc:subject", "A different subject"),
    ("creator", "dc:creator", "Créatrice 😀"),
    ("description", "dc:description", "A description"),
    ("keywords", "cp:keywords", "one; two"),
    ("category", "cp:category", "Review"),
    ("version", "cp:version", "07.02"),
    ("created", "dcterms:created", "2024-02-29T04:05:06+02:00"),
    ("modified", "dcterms:modified", "2025-06-03T04:05:06Z"),
    ("lastPrinted", "cp:lastPrinted", "2025-02-03T04:05:06Z"),
    ("lastModifiedBy", "cp:lastModifiedBy", "Editor"),
    ("revision", "cp:revision", "0007"),
    ("identifier", "dc:identifier", "urn:example:report"),
    ("language", "dc:language", "fr-CA"),
    ("contentStatus", "cp:contentStatus", ""),
]
EXTENDED_FIELDS = [
    ("company", "Company", "Example & Co."),
    ("manager", "Manager", "Manager 😀"),
]


def populate_metadata(document):
    expected = {}
    core = document.part.package.part_related_by(RT.CORE_PROPERTIES)
    core.partname = PackURI("/docProps/resources/core.xml")
    for field, tag, text in CORE_FIELDS:
        node = core.element.find(qn(tag))
        if node is None:
            node = OxmlElement(tag)
            core.element.append(node)
        node.text = expected[field] = text
    extended = document.part.package.part_related_by(RT.EXTENDED_PROPERTIES)
    extended.partname = PackURI("/docProps/resources/app.xml")
    props = etree.fromstring(extended.blob)
    for field, tag, text in EXTENDED_FIELDS:
        name = f"{{{EXT}}}{tag}"
        node = props.find(name)
        if node is None:
            node = etree.SubElement(props, name)
        node.text = expected[field] = text
    extended._blob = etree.tostring(props)
    return dict(expected, comments=expected["description"])


def populate_defaults(document):
    run, paragraph = word_defaults(document)
    run.clear()
    paragraph.clear()
    settings = document.part.part_related_by(RT.SETTINGS).element
    expected = {}

    def add(parent, tag, **attributes):
        node = parent.find(qn(f"w:{tag}"))
        if node is None:
            node = OxmlElement(f"w:{tag}")
            parent.append(node)
        for attr, (field, value) in attributes.items():
            expected[field] = value
            if value is True and attr == "val":
                continue  # No w:val on a standalone toggle means true.
            text = "off" if value is False else "on" if value is True else str(value)
            node.set(qn(f"w:{attr}"), text)
        return node

    def toggles(parent, *properties):
        for index, (tag, field) in enumerate(properties):
            add(parent, tag, val=(field, index % 2 == 0))

    toggles(run, ("b", "bold"), ("i", "italic"), ("bCs", "boldCs"), ("iCs", "italicCs"),
            ("rtl", "runRtl"), ("cs", "complexScript"), ("caps", "caps"),
            ("smallCaps", "smallCaps"), ("strike", "strike"), ("snapToGrid", "runSnapToGrid"))
    add(run, "rFonts", ascii=("fontAscii", "Liberation Serif"), hAnsi=("fontHAnsi", "Carlito"),
        eastAsia=("fontEastAsia", "MS Mincho"), cs=("fontCs", "Arial"),
        asciiTheme=("fontAsciiTheme", "majorAscii"), hAnsiTheme=("fontHAnsiTheme", "minorHAnsi"),
        eastAsiaTheme=("fontEastAsiaTheme", "majorEastAsia"), cstheme=("fontCsTheme", "minorBidi"))
    add(run, "sz", val=("sizeHalfPoints", 23))
    add(run, "szCs", val=("sizeCsHalfPoints", 31))
    add(run, "lang", val=("language", "en-GB"), eastAsia=("languageEastAsia", "ja-JP"),
        bidi=("languageBidi", "ar-SA"))
    add(run, "u", val=("underline", "wave"))
    add(run, "vertAlign", val=("verticalAlign", "subscript"))
    add(run, "color", val=("runColor", "ab23CD"), themeColor=("colorTheme", "accent4"),
        themeTint=("colorTint", "a2"), themeShade=("colorShade", "B3"))

    toggles(paragraph, ("bidi", "paragraphBidi"), ("keepLines", "keepTogether"),
            ("keepNext", "keepWithNext"), ("pageBreakBefore", "pageBreakBefore"),
            ("widowControl", "widowControl"), ("suppressLineNumbers", "suppressLineNumbers"),
            ("contextualSpacing", "contextualSpacing"), ("mirrorIndents", "mirrorIndents"),
            ("snapToGrid", "paragraphSnapToGrid"))
    add(paragraph, "spacing", before=("beforeTwips", 201), after=("afterTwips", 203),
        beforeLines=("beforeLines", 104), afterLines=("afterLines", 105),
        beforeAutospacing=("beforeAutoSpacing", False), afterAutospacing=("afterAutoSpacing", True),
        line=("line", 247), lineRule=("lineRule", "exact"))
    add(paragraph, "ind", left=("leftTwips", -301), right=("rightTwips", 302),
        start=("startTwips", 303), end=("endTwips", -304), firstLine=("firstLineTwips", 305),
        hanging=("hangingTwips", 306), leftChars=("leftChars", 107), rightChars=("rightChars", -108),
        startChars=("startChars", 109), endChars=("endChars", -110),
        firstLineChars=("firstLineChars", 111), hangingChars=("hangingChars", 112))
    add(paragraph, "jc", val=("justification", "both"))
    tabs = add(paragraph, "tabs")
    expected["tabStops"] = [
        dict(position_twips=720, alignment="decimal", leader="dot"),
        dict(position_twips=-240, alignment="left", leader=None),
    ]
    for tab in expected["tabStops"]:
        node = OxmlElement("w:tab")
        node.set(qn("w:pos"), str(tab["position_twips"]))
        node.set(qn("w:val"), tab["alignment"])
        if tab["leader"] is not None:
            node.set(qn("w:leader"), tab["leader"])
        tabs.append(node)

    toggles(settings, ("mirrorMargins", "mirrorMargins"), ("gutterAtTop", "gutterAtTop"),
            ("trackRevisions", "trackRevisions"))
    for kind, (position, fmt, start, restart) in [
        ("footnote", ("beneathText", "lowerRoman", 3, "eachPage")),
        ("endnote", ("sectEnd", "upperLetter", 7, "eachSect")),
    ]:
        note = add(settings, f"{kind}Pr")
        add(note, "pos", val=(f"{kind}Position", position))
        add(note, "numFmt", val=(f"{kind}Format", fmt))
        add(note, "numStart", val=(f"{kind}Start", start))
        add(note, "numRestart", val=(f"{kind}Restart", restart))
    return expected


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
    native = native_read(source, "metadata", "description", "comments", "title", "identifier")
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=dict(kind="resources"))
        for field in ("description", "comments"):
            await assert_readback(client, document, resource_query("metadata", field=field),
                                  native[field], "ambiguous dc:description", error=True)
        await failure(client, "inspect_document", "inspection_failed", document=document,
                      view=resource_query("custom-property", name="LegacyNumber"))
        await failure(client, "inspect_document", "invalid_input", document=document,
                      view=resource_query("default-alignment", extra=True))

        async def value(kind, **selector):
            result = await call(client, "inspect_document", document=document,
                                view=resource_query(kind, **selector))
            assert result["document"] == document
            return result["value"]

        # Title shares the malformed description's physical part.
        for field, expected in [("title", "Initial title"), ("identifier", None)]:
            await assert_readback(client, document, resource_query("metadata", field=field),
                                  native[field], expected)
        assert await value("custom-property", name="Reviewed") == "pending"
        assert await value("custom-property", name="Disabled") is False
        assert await value("custom-property", name="Blank") == ""
        assert await value("custom-property", name="blank") is None
        assert await value("default-alignment") == "center"
        assert (await value("document-defaults"))["bold"] is True


async def test_all_modeled_metadata_and_defaults_match_native(launch, tmp_path):
    fixture = build_source()
    metadata = populate_metadata(fixture)
    defaults = populate_defaults(fixture)
    source = tmp_path / "all-resource-values.docx"
    fixture.save(source)

    native_metadata = native_read(source, "metadata", *metadata)
    native_defaults = native_read(source, "document-defaults")
    async with launch() as client:
        document = (await call(client, "open_document", path=source.name))["document"]
        for field, expected in metadata.items():
            await assert_readback(client, document, resource_query("metadata", field=field),
                                  native_metadata[field], expected)
        await assert_readback(client, document, resource_query("document-defaults"),
                              native_defaults, defaults)


async def test_modeled_resource_set_and_clear_native_bytes(launch, tmp_path):
    fixture = build_source()
    for properties in word_defaults(fixture):
        properties.clear()  # Ensure each advertised default set is a real change.
    source = tmp_path / "source.docx"
    fixture.save(source)

    # Author the target values independently; the oracle reads these as native
    # types instead of decoding the MCP edit payloads.
    fixture = build_source()
    defaults = populate_defaults(fixture)
    metadata = populate_metadata(fixture)
    metadata["created"] = "2024-02-29T02:05:06Z"  # Only UTC timestamps are writable.
    fixture.core_properties._element.find(qn("dcterms:created")).text = metadata["created"]
    values = tmp_path / "values.docx"
    fixture.save(values)
    # Comments aliases description; do not target the same leaf twice.
    fields = [field for field in metadata if field != "comments"]

    async with launch() as client:
        schema = next(tool.inputSchema for tool in (await client.list_tools()).tools
                      if tool.name == "preview_edits")
        validator = validator_for(schema)(schema)
        document = (await call(client, "open_document", path=source.name))["document"]
        initial = await read_resources(client, document)
        assert not initial["document_defaults"].keys() & defaults.keys()
        assert all(initial["metadata"][field] != metadata[field] for field in fields)

        for phase in ("set", "clear"):
            setting = phase == "set"
            edits = [
                resource(kind, field=field, value=payload[field] if setting else None)
                for kind, payload, names in [
                    ("default", defaults, defaults),
                    ("metadata", metadata, fields),
                ]
                for field in names
            ]
            alias = resource("metadata", field="comments",
                             value=metadata["comments"] if setting else None)
            for edit in [*edits, alias]:
                assert validator.is_valid(dict(document=document, edits=[edit])), (phase, edit)

            output = tmp_path / f"{phase}.docx"
            original, before = source.read_bytes(), parts(source)
            preview = await call(client, "preview_edits", document=document, edits=edits)
            saved = await call(client, "save_copy", document=document, preview=preview["preview"],
                               output=output.name)
            assert source.read_bytes() == original
            native = tmp_path / f"native-{phase}.docx"
            resource_oracle("write-modeled", native, source, values, phase, *fields, cwd=tmp_path)
            assert output.read_bytes() == native.read_bytes()
            assert saved["output_snapshot"] == preview["output_snapshot"] == digest(
                output.read_bytes(),
            )
            after = parts(output)
            assert before.keys() == after.keys()
            assert {name for name in before if before[name] != after[name]} == {
                STYLES, SETTINGS, CORE, EXTENDED,
            }
            assert etree.fromstring(after[SETTINGS]).find(qn("w:zoom")).get(
                qn("w:percent"),
            ) == "100"

            document = (await call(client, "open_document", path=output.name))["document"]
            readback = await read_resources(client, document)
            assert readback["metadata"] == (metadata if setting else dict.fromkeys(metadata))
            await assert_readback(client, document, resource_query("document-defaults"),
                                  native_read(native, "document-defaults"),
                                  defaults if setting else {})
            source = output

    validate(tmp_path / "source.docx", tmp_path / "set.docx", tmp_path / "clear.docx")


async def test_direct_and_legacy_alignment_readback_match_native(launch, tmp_path):
    cases = [
        ("left", "left", None, None),
        ("center", "center", None, None),
        ("right", "right", None, None),
        ("both", "justify", None, None),
        ("distribute", "distribute", None, None),
        ("mediumKashida", "justify-medium", None, None),
        ("highKashida", "justify-high", None, None),
        ("lowKashida", "justify-low", None, None),
        ("thaiDistribute", "thai-justify", None, None),
        *((token, None, "unsupported default alignment", None)
          for token in ("start", "end", "numTab")),
        (None, None, None, None),
        ("unknown", None, "unsupported default alignment", "invalid or unsupported document default"),
        ("", None, "missing w:val", "invalid or unsupported document default"),
        ("annotated", None, "annotated resource property", None),
        ("duplicate", None, "ambiguous w:jc", "ambiguous w:jc"),
    ]
    async with launch() as client:
        for index, (token, legacy, legacy_error, defaults_error) in enumerate(cases):
            fixture = build_source()
            _, paragraph = word_defaults(fixture)
            jc = paragraph.find(qn("w:jc"))
            direct = token
            if token is None:
                paragraph.remove(jc)
            elif token == "":
                del jc.attrib[qn("w:val")]
            elif token == "annotated":
                jc.set("{urn:fixture}annotation", "retain")
                direct = "center"
            elif token == "duplicate":
                paragraph.append(OxmlElement("w:jc"))
            else:
                jc.set(qn("w:val"), token)
            source = tmp_path / f"alignment-{index}.docx"
            fixture.save(source)

            document = (await call(client, "open_document", path=source.name))["document"]
            await assert_readback(client, document, resource_query("default-alignment"),
                                  native_read(source, "default-alignment"),
                                  legacy_error or legacy, error=legacy_error is not None)
            native = native_read(source, "document-defaults")
            if defaults_error:
                await assert_readback(client, document, resource_query("document-defaults"),
                                      native, defaults_error, error=True)
            else:
                # Check this leaf independently; the full map is qualified separately.
                assert native["value"].get("justification") == direct
                assert ("justification" in native["value"]) == (direct is not None)
                await assert_readback(client, document, resource_query("document-defaults"),
                                      native, native["value"])
            await call(client, "close_document", document=document)


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
