"""Independent XSD validation using official, edition-pinned schemas (dev only)."""
import importlib.util
import io
import os
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

from lxml import etree
import pytest
from rocx import declarative as d

ROOT = Path(__file__).parents[2]
spec = importlib.util.spec_from_file_location("ooxml_validator", ROOT / "tools/validate_ooxml.py")
validation = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validation)


@pytest.fixture(scope="module")
def validator():
    directory = validation.default_directory()
    if not (directory / "wml.xsd").exists():
        if os.environ.get("ROCX_REQUIRE_SCHEMAS") == "1":
            pytest.fail("Required official schemas missing; run tools/validate_ooxml.py --download-schemas")
        pytest.skip("Download pinned schemas with tools/validate_ooxml.py --download-schemas or set ROCX_SCHEMA_DIR")
    return validation.Validator(directory)


@pytest.fixture
def chart_package():
    """Hand-authored workbook-free chart, independent of the native codec."""
    c, w, a, r = validation.C, validation.W, validation.A, validation.R
    rel, ct = validation.REL, validation.CT
    wp = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
    return {name: etree.fromstring(xml) for name, xml in {
        "[Content_Types].xml": f"""
            <Types xmlns="{ct}">
              <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
              <Override PartName="/word/document.xml"
                        ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
              <Default Extension="chart" ContentType="{validation.CHART_TYPE}"/>
            </Types>""",
        "_rels/.rels": f"""
            <Relationships xmlns="{rel}">
              <Relationship Id="rId1" Type="{r}/officeDocument" Target="word/document.xml"/>
            </Relationships>""",
        "word/_rels/document.xml.rels": f"""
            <Relationships xmlns="{rel}">
              <Relationship Id="rId2" Type="{r}/chart" Target="charts/column.chart"/>
            </Relationships>""",
        "word/document.xml": f"""
            <w:document xmlns:w="{w}" xmlns:a="{a}" xmlns:wp="{wp}" xmlns:c="{c}" xmlns:r="{r}">
              <w:body><w:p><w:r><w:drawing><wp:inline>
                <wp:extent cx="1828800" cy="914400"/>
                <wp:docPr id="1" name="Column chart"/>
                <a:graphic><a:graphicData uri="{c}"><c:chart r:id="rId2"/></a:graphicData></a:graphic>
              </wp:inline></w:drawing></w:r></w:p></w:body>
            </w:document>""",
        "word/charts/column.chart": f"""
            <chartSpace xmlns="{c}"><chart><plotArea>
              <barChart><barDir val="col"/>
                <ser><idx val="0"/><order val="0"/>
                  <cat><strLit><ptCount val="1"/><pt idx="0"><v>North</v></pt></strLit></cat>
                  <val><numLit><ptCount val="1"/><pt idx="0"><v>8</v></pt></numLit></val>
                </ser>
                <axId val="101"/><axId val="102"/>
              </barChart>
              <catAx><axId val="101"/><scaling/><axPos val="b"/><crossAx val="102"/></catAx>
              <valAx><axId val="102"/><scaling/><axPos val="l"/><crossAx val="101"/></valAx>
            </plotArea></chart></chartSpace>""",
    }.items()}


def zip_xml_parts(parts):
    output = io.BytesIO()
    with ZipFile(output, "w", ZIP_DEFLATED) as package:
        for name, root in parts.items():
            package.writestr(name, etree.tostring(root))
    return output.getvalue()


@pytest.mark.parametrize("use_override", [False, True], ids=["default", "override"])
def test_literal_chart_part_and_owner_reference_validate(validator, chart_package, use_override):
    chart_path = "word/charts/column.chart"
    if use_override:
        path = "word/charts/chart1.xml"
        chart_package[path] = chart_package.pop(chart_path)
        chart_package["word/_rels/document.xml.rels"][0].set("Target", "charts/chart1.xml")
        etree.SubElement(chart_package["[Content_Types].xml"], f"{{{validation.CT}}}Override",
                         PartName=f"/{path}", ContentType=validation.CHART_TYPE)
        chart_path = path
    assert set(validator.validate(zip_xml_parts(chart_package))) == {"word/document.xml", chart_path}


@pytest.mark.parametrize(("mutation", "diagnostic"), [
    ("invalid bar direction", r"word/charts/column\.chart: .*barDir"),
    ("wrong root", r"incorrect chart root .*column\.chart"),
    ("wrong content type", r"incorrect chart content type .*column\.chart"),
    ("missing target", r"missing relationship target .*column\.chart"),
    ("external relationship", r"chart relationship .* must be internal"),
    ("wrong relationship type", r"missing owner-local chart relationship"),
    ("wrong relationship owner", r"missing owner-local chart relationship"),
    ("missing reference ID", r"missing owner-local chart relationship"),
    ("wrong payload URI", r"incorrect chart graphicData URI"),
    ("empty payload", r"expected one chart reference"),
])
def test_invalid_chart_package_fails(validator, chart_package, mutation, diagnostic):
    c, a, r = validation.C, validation.A, validation.R
    chart_path = "word/charts/column.chart"
    chart = chart_package[chart_path]
    payload = chart_package["word/document.xml"].find(f".//{{{a}}}graphicData")
    reference = payload[0]
    owner = chart_package["word/_rels/document.xml.rels"]
    relation = owner[0]
    if mutation == "invalid bar direction":
        chart.find(f".//{{{c}}}barDir").set("val", "invalid")
    elif mutation == "wrong root":
        chart_package[chart_path] = etree.Element(f"{{{c}}}chart", {f"{{{r}}}id": "rId2"})
    elif mutation == "wrong content type":
        etree.SubElement(chart_package["[Content_Types].xml"], f"{{{validation.CT}}}Override",
                         PartName=f"/{chart_path}", ContentType="application/xml")
    elif mutation == "missing target":
        del chart_package[chart_path]
    elif mutation == "external relationship":
        relation.set("TargetMode", "External")
        relation.set("Target", "https://example.test/chart")
    elif mutation == "wrong relationship type":
        relation.set("Type", f"{r}/image")
    elif mutation == "wrong relationship owner":
        relation.set("Target", chart_path)
        chart_package["_rels/.rels"].append(relation)
    elif mutation == "missing reference ID":
        del reference.attrib[f"{{{r}}}id"]
    elif mutation == "wrong payload URI":
        payload.set("uri", a)
    elif mutation == "empty payload":
        payload.remove(reference)
    else:
        pytest.fail(f"unknown mutation {mutation}")
    with pytest.raises(validation.ValidationError, match=diagnostic):
        validator.validate(zip_xml_parts(chart_package))


async def test_inline_fraction_requires_a_denominator(validator):
    from test_inline_fraction import NS, document
    from test_notes import rewrite

    data = await document(d.InlineFraction(" 1 ", "2")).to_bytes()
    assert "word/document.xml" in validator.validate(data)

    def invalid(root):
        denominator, = root.xpath("//m:oMath/m:f/m:den", namespaces=NS)
        denominator.getparent().remove(denominator)

    with pytest.raises(validation.ValidationError, match="den"):
        validator.validate(rewrite(data, "word/document.xml", invalid))


async def test_custom_properties_validate_at_a_nonstandard_path(validator):
    from test_custom_properties import relocate

    data = await d.Document([], custom_properties={"Project": "Atlas"}).to_bytes()
    path = "metadata/project.props"
    assert path in validator.validate(relocate(data, etree.tostring, path))


async def test_custom_boolean_requires_schema_boolean_lexical_value(validator):
    from test_custom_properties import DEFAULT, VT
    from test_notes import rewrite

    data = await d.Document([], custom_properties={"Approved": False}).to_bytes()
    assert DEFAULT in validator.validate(data)

    def invalid(root):
        root[0].find(f"{{{VT}}}bool").text = "no"

    with pytest.raises(validation.ValidationError, match="not a valid value.*boolean"):
        validator.validate(rewrite(data, DEFAULT, invalid))


@pytest.mark.parametrize(("mutation", "diagnostic"), [
    ("wrong content type", "incorrect custom properties content type"),
    ("wrong root", "incorrect custom properties root"),
    ("duplicate ID", "duplicate custom property ID"),
    ("reserved ID", "invalid or duplicate custom property ID"),
    ("missing relationship", "no package-level relationship"),
    ("external relationship", "must be internal and package-level"),
    ("duplicate relationship", "multiple package-level custom properties relationships"),
])
async def test_invalid_custom_properties_package_fails(validator, mutation, diagnostic):
    from copy import deepcopy
    from test_notes import rewrite

    data = await d.Document([], custom_properties={"Project": "Atlas", "Review": "Draft"}).to_bytes()
    part_path = {
        "wrong content type": "[Content_Types].xml",
        "wrong root": "docProps/custom.xml",
        "duplicate ID": "docProps/custom.xml",
        "reserved ID": "docProps/custom.xml",
    }.get(mutation, "_rels/.rels")

    def invalid(root):
        if mutation == "wrong content type":
            part = next(child for child in root if child.get("PartName") == "/docProps/custom.xml")
            part.set("ContentType", "application/xml")
        elif mutation == "wrong root":
            root.tag = f"{{{validation.W}}}document"
        elif mutation in {"duplicate ID", "reserved ID"}:
            root[1].set("pid", root[0].get("pid") if mutation == "duplicate ID" else "1")
        else:
            relation = next(child for child in root if child.get("Type") == validation.CUSTOM_REL)
            if mutation == "missing relationship":
                root.remove(relation)
            elif mutation == "external relationship":
                relation.set("TargetMode", "External")
                relation.set("Target", "https://example.test/custom")
            elif mutation == "duplicate relationship":
                clone = deepcopy(relation)
                clone.set("Id", "extraCustom")
                root.append(clone)
            else:
                pytest.fail(f"unknown mutation {mutation}")

    with pytest.raises(validation.ValidationError, match=diagnostic):
        validator.validate(rewrite(data, part_path, invalid))


async def test_extended_properties_validate_at_a_nonstandard_path(validator):
    from test_extended_properties import move_app

    data = await d.Document([], company="Atlas").to_bytes()
    path = "metadata/extension.props"
    assert path in validator.validate(move_app(data, path))


@pytest.mark.parametrize("mutation", ["external", "missing", "wrong content type"])
async def test_invalid_extended_properties_relationship_fails(validator, mutation):
    from test_notes import rewrite

    data = await d.Document([], manager="Taylor").to_bytes()
    if mutation == "wrong content type":
        def invalid(root):
            part = next(child for child in root if child.get("PartName") == "/docProps/app.xml")
            part.set("ContentType", validation.CUSTOM_TYPE)

        data = rewrite(data, "[Content_Types].xml", invalid)
        diagnostic = "incorrect extended properties content type"
    else:
        def invalid(root):
            relation = next(child for child in root if child.get("Type") == validation.EXTENDED_REL)
            if mutation == "missing":
                root.remove(relation)
            else:
                relation.set("TargetMode", "External")
                relation.set("Target", "https://example.test/extended")

        data = rewrite(data, "_rels/.rels", invalid)
        diagnostic = "extended properties .* (has no package-level relationship|must be internal)"
    with pytest.raises(validation.ValidationError, match=diagnostic):
        validator.validate(data)


async def test_all_features_visual_catalog_passes_schema_and_bad_border_order_fails(validator, tmp_path):
    import test_notes
    spec = importlib.util.spec_from_file_location("visual_catalog", ROOT / "examples/all_features.py")
    example = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(example)
    data = await example.generate(tmp_path / "all-features.docx")
    checked = validator.validate(data)
    assert {"word/charts/chart1.xml", "docProps/custom.xml", "docProps/app.xml"} <= set(checked)
    def duplicate_company(root):
        etree.SubElement(root, f"{{{validation.EXTENDED}}}Company").text = "Duplicate"
    with pytest.raises(validation.ValidationError, match="docProps/app.xml"):
        validator.validate(test_notes.rewrite(data, "docProps/app.xml", duplicate_company))
    def invalid_custom_pid(root):
        root[0].set("pid", "not-an-integer")
    with pytest.raises(validation.ValidationError, match="docProps/custom.xml"):
        validator.validate(test_notes.rewrite(data, "docProps/custom.xml", invalid_custom_pid))
    def invalid_order(root):
        borders = root.find(f".//{{{validation.W}}}tcBorders")
        borders[:] = list(reversed(borders))
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid_order))
    def invalid_anchor_order(root):
        namespace = "{http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing}"
        anchor = root.find(f".//{namespace}anchor")
        position = anchor.find(f"{namespace}positionV")
        anchor.remove(position)
        anchor.insert(0, position)
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid_anchor_order))
    def missing_floating_text_box_wrap(root):
        ns = {"wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
              "a": "http://schemas.openxmlformats.org/drawingml/2006/main"}
        anchor, = root.xpath("//wp:anchor[.//a:txSp]", namespaces=ns)
        anchor.remove(anchor.find("wp:wrapSquare", ns))
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", missing_floating_text_box_wrap))
    def invalid_shape_font_size(root):
        namespace = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
        properties = root.find(f".//{namespace}txBody/{namespace}p/{namespace}r/{namespace}rPr")
        properties.set("sz", "99")
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid_shape_font_size))
    def invalid_shape_property_order(root):
        namespace = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
        paragraph = root.find(f".//{namespace}txBody/{namespace}p")
        properties = paragraph.find(f"{namespace}pPr")
        paragraph.remove(properties)
        paragraph.append(properties)
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid_shape_property_order))
    def invalid_shape_typeface_order(root):
        namespace = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
        properties = root.find(f".//{namespace}txBody/{namespace}p/{namespace}r/{namespace}rPr")
        properties[:] = list(reversed(properties))
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid_shape_typeface_order))
    def missing_typeface(root):
        namespace = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
        del root.find(f".//{namespace}txBody/{namespace}p/{namespace}r/{namespace}rPr/{namespace}latin").attrib["typeface"]
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", missing_typeface))
    def missing_text_box_transform(root):
        namespace = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
        canvas = next(n for n in root.iter() if etree.QName(n).localname == "lockedCanvas" and n.find(f"{namespace}txSp") is not None)
        text = canvas.find(f"{namespace}txSp")
        text.remove(text.find(f"{namespace}xfrm"))
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", missing_text_box_transform))
    def invalid_wrap_polygon(root):
        namespace = "{http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing}"
        polygon = root.find(f".//{namespace}wrapPolygon")
        polygon.remove(polygon.findall(f"{namespace}lineTo")[-1])
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid_wrap_polygon))
    def invalid_page_border_order(root):
        borders = root.find(f".//{{{validation.W}}}pgBorders")
        borders[:] = list(reversed(borders))
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid_page_border_order))
    def invalid_line_restart(root):
        line = root.find(f".//{{{validation.W}}}lnNumType")
        line.set(f"{{{validation.W}}}restart", "invalid")
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid_line_restart))
    def invalid_level_properties_order(root):
        namespace = f"{{{validation.W}}}"
        level = next(level for level in root.iter(f"{namespace}lvl")
                     if level.find(f"{namespace}pPr") is not None
                     and level.find(f"{namespace}rPr") is not None)
        run = level.find(f"{namespace}rPr")
        level.remove(run)
        level.insert(0, run)
    with pytest.raises(validation.ValidationError, match="word/numbering.xml"):
        validator.validate(test_notes.rewrite(data, "word/numbering.xml", invalid_level_properties_order))
    def missing_revision_author(root):
        insertion = root.find(f".//{{{validation.W}}}ins")
        del insertion.attrib[f"{{{validation.W}}}author"]
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", missing_revision_author))
    def invalid_conditional_order(root):
        style = next(n for n in root if n.get(f"{{{validation.W}}}styleId") == "Regionfirst_row")
        properties = style.find(f"{{{validation.W}}}tblStylePr")
        properties[:] = list(reversed(properties))
    with pytest.raises(validation.ValidationError, match="word/styles.xml"):
        validator.validate(test_notes.rewrite(data, "word/styles.xml", invalid_conditional_order))


async def test_table_look_passes_schema_and_bad_boolean_or_mask_fails(validator):
    import test_notes
    def table():
        return d.Table([d.TableRow([d.TableCell([d.Paragraph(text="look")])])],
            look=d.TableLook(last_row=True, last_column=True, vertical_banding=True))
    data = await d.Document([d.Section([table()], headers={"default": d.Header([table()])})]).to_bytes()
    assert "word/document.xml" in validator.validate(data)
    for attribute, value in [("lastRow", "maybe"), ("val", "ZZZZ")]:
        def invalid(root):
            root.find(f".//{{{validation.W}}}tblLook").set(f"{{{validation.W}}}{attribute}", value)
        with pytest.raises(validation.ValidationError, match="word/document.xml"):
            validator.validate(test_notes.rewrite(data, "word/document.xml", invalid))


async def test_section_direction_passes_schema_and_invalid_flag_fails(validator):
    import test_notes
    data = await d.Document([
        d.Section([d.Paragraph(text="rtl section", bidi=False)], properties=d.SectionProperties(bidi=True)),
        d.Section([d.Paragraph(text="ltr section")], properties=d.SectionProperties(bidi=False)),
    ]).to_bytes()
    assert "word/document.xml" in validator.validate(data)
    def invalid(root):
        root.find(f".//{{{validation.W}}}sectPr/{{{validation.W}}}bidi").set(f"{{{validation.W}}}val", "maybe")
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid))


async def test_conditional_paragraph_properties_pass_schema_and_bad_values_fail(validator):
    import test_notes
    import test_conditional_paragraph_formatting
    formatting = d.TableSectionFormat(line_height=d.LineHeight.exact(14.5),
        tab_stops=[d.TabStop(72, "right", "dot")], bidi=False)
    data = await d.Document([], styles=[d.TableStyleDefinition(name="Conditional",
        **{region: formatting for region, _ in test_conditional_paragraph_formatting.REGIONS})]).to_bytes()
    assert "word/styles.xml" in validator.validate(data)
    for element, attribute, value in [("spacing", "lineRule", "unknown"), ("tab", "val", "unknown"), ("bidi", "val", "maybe")]:
        def invalid(root):
            style = next(n for n in root if n.get(f"{{{validation.W}}}styleId") == "Conditional")
            region = next(n for n in style.findall(f"{{{validation.W}}}tblStylePr") if n.get(f"{{{validation.W}}}type") == "firstRow")
            region.find(f".//{{{validation.W}}}{element}").set(f"{{{validation.W}}}{attribute}", value)
        with pytest.raises(validation.ValidationError, match="word/styles.xml"):
            validator.validate(test_notes.rewrite(data, "word/styles.xml", invalid))


async def test_conditional_script_properties_pass_schema_and_bad_values_fail(validator):
    import test_notes
    import test_conditional_paragraph_formatting
    formatting = d.TableSectionFormat(language=d.RunLanguage("en-US", "ja-JP", "ar-SA"),
        bold_complex_script=False, italic_complex_script=True, size_complex_script=13.5,
        rtl=False, complex_script=True, font_east_asia="East", font_complex_script="Complex")
    data = await d.Document([], styles=[d.TableStyleDefinition(name="Scripts",
        **{region: formatting for region, _ in test_conditional_paragraph_formatting.REGIONS})]).to_bytes()
    assert "word/styles.xml" in validator.validate(data)
    for element, value in [("bCs", "maybe"), ("szCs", "-1"), ("rtl", "maybe")]:
        def invalid(root):
            style = next(n for n in root if n.get(f"{{{validation.W}}}styleId") == "Scripts")
            properties = style.find(f"{{{validation.W}}}rPr")
            properties.find(f"{{{validation.W}}}{element}").set(f"{{{validation.W}}}val", value)
        with pytest.raises(validation.ValidationError, match="word/styles.xml"):
            validator.validate(test_notes.rewrite(data, "word/styles.xml", invalid))


async def test_document_defaults_pass_schema_and_invalid_values_and_order_fail(validator, tmp_path):
    import test_notes
    data = await d.Document([], defaults=d.DocumentDefaults(run=d.RunDefaults(font="Literal", size=13.5, rtl=False),
        paragraph=d.ParagraphDefaults(bidi=True, line_height=d.LineHeight.exact(14),
            first_line_indent=-6, tab_stops=[d.TabStop(36)]))).to_bytes()
    assert "word/styles.xml" in validator.validate(data)
    def invalid_size(root):
        root.find(f"{{{validation.W}}}docDefaults/{{{validation.W}}}rPrDefault/{{{validation.W}}}rPr/{{{validation.W}}}sz").set(f"{{{validation.W}}}val", "-1")
    def invalid_order(root):
        defaults = root.find(f"{{{validation.W}}}docDefaults")
        defaults[:] = list(reversed(defaults))
    for mutate in [invalid_size, invalid_order]:
        with pytest.raises(validation.ValidationError, match="word/styles.xml"):
            validator.validate(test_notes.rewrite(data, "word/styles.xml", mutate))
    def remove_defaults(root):
        root.remove(root.find(f"{{{validation.W}}}docDefaults"))
    template = tmp_path / "no-defaults.docx"
    template.write_bytes(test_notes.rewrite(data, "word/styles.xml", remove_defaults))
    new_defaults = await d.Document([], defaults=d.DocumentDefaults(paragraph=d.ParagraphDefaults(bidi=False),
        run=d.RunDefaults(size=0))).to_bytes(d.DocumentTemplate(template))
    assert "word/styles.xml" in validator.validate(new_defaults)


async def test_linked_styles_pass_schema_and_a_missing_link_value_fails(validator):
    import test_notes
    data = await d.Document([], styles=[
        d.ParagraphStyleDefinition(name="Pair Paragraph", linked_style="Pair Character"),
        d.CharacterStyleDefinition(name="Pair Character", linked_style="Pair Paragraph"),
    ]).to_bytes()
    assert "word/styles.xml" in validator.validate(data)
    def missing_value(root):
        style = next(n for n in root if n.get(f"{{{validation.W}}}styleId") == "PairParagraph")
        del style.find(f"{{{validation.W}}}link").attrib[f"{{{validation.W}}}val"]
    with pytest.raises(validation.ValidationError, match="word/styles.xml"):
        validator.validate(test_notes.rewrite(data, "word/styles.xml", missing_value))


async def test_section_gutter_passes_schema_and_invalid_boolean_fails(validator):
    import test_notes
    data = await d.Document([d.Section([d.Paragraph(text="gutter")],
        properties=d.SectionProperties(rtl_gutter=True, bidi=False))]).to_bytes()
    assert "word/document.xml" in validator.validate(data)
    def invalid_boolean(root):
        root.find(f".//{{{validation.W}}}sectPr/{{{validation.W}}}rtlGutter").set(f"{{{validation.W}}}val", "maybe")
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid_boolean))


async def test_official_schema_accepts_native_content_and_detects_invalid_order(validator):
    document = d.Document([d.Section(children=[d.Paragraph(text="schema", style="Normal", alignment="center")])])
    data = await document.to_bytes()
    assert "word/document.xml" in validator.validate(data)
    output = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source, ZipFile(output, "w", ZIP_DEFLATED) as target:
        for name in source.namelist():
            payload = source.read(name)
            if name == "word/document.xml":
                tree = etree.fromstring(payload)
                properties = tree.find(f".//{{{validation.W}}}pPr")
                properties[:] = list(reversed(properties))
                payload = etree.tostring(tree)
            target.writestr(name, payload)
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(output.getvalue())


async def test_official_schema_accepts_features_and_rejects_missing_relationship_targets(validator):
    image = ROOT / "tests/fixtures/images/sample.png"
    document = d.Document([d.Section(children=[d.Paragraph(children=[d.ImageRun(data=image)])],
        headers={"even": d.Header(children=[d.Paragraph(text="even")])})])
    data = await document.to_bytes()
    assert len(validator.validate(data)) >= 5
    output = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source, ZipFile(output, "w", ZIP_DEFLATED) as target:
        for name in source.namelist():
            payload = source.read(name)
            if name == "word/_rels/document.xml.rels":
                payload = payload.replace(b"media/image1.png", b"media/missing.png")
            target.writestr(name, payload)
    with pytest.raises(validation.ValidationError, match="missing relationship target"):
        validator.validate(output.getvalue())


async def test_typed_line_heights_pass_schemas_and_invalid_rule_is_rejected(validator):
    import test_notes
    data = await d.Document([d.Section(children=[
        d.Paragraph(text="exact", line_height=d.LineHeight.exact(14.25), style="Minimum"),
        d.Paragraph(text="multiple", line_height=d.LineHeight.multiple(1.15)),
        d.Paragraph(text="minimum", line_height=d.LineHeight.at_least(18)),
    ])], styles=[d.ParagraphStyleDefinition(name="Minimum", line_height=d.LineHeight.at_least(20))]).to_bytes()
    assert {"word/document.xml", "word/styles.xml"} <= set(validator.validate(data))
    def invalid(root):
        root.find(f".//{{{validation.W}}}spacing").set(f"{{{validation.W}}}lineRule", "minimum")
    for owner in ["word/document.xml", "word/styles.xml"]:
        with pytest.raises(validation.ValidationError, match=owner):
            validator.validate(test_notes.rewrite(data, owner, invalid))


async def test_custom_tab_stops_pass_schemas_and_bad_alignment_leader_or_position_fails(validator):
    import test_notes
    stops = [d.TabStop(-12, "clear"), d.TabStop(36, "left", "none"), d.TabStop(72, "center", "dot"),
        d.TabStop(108, "right", "hyphen"), d.TabStop(144, "decimal", "underscore"), d.TabStop(180, "bar", "heavy"),
        d.TabStop(216, "start", "middle_dot"), d.TabStop(252, "end")]
    data = await d.Document([d.Section(children=[d.Paragraph(text="schema", tab_stops=stops, style="Tabbed")])],
        styles=[d.ParagraphStyleDefinition(name="Tabbed", tab_stops=stops)]).to_bytes()
    assert {"word/document.xml", "word/styles.xml"} <= set(validator.validate(data))
    for owner in ["word/document.xml", "word/styles.xml"]:
        for attribute, value in [("val", "unknown"), ("leader", "dots"), ("pos", "not-a-number")]:
            def invalid(root):
                root.find(f".//{{{validation.W}}}tabs/{{{validation.W}}}tab").set(f"{{{validation.W}}}{attribute}", value)
            with pytest.raises(validation.ValidationError, match=owner):
                validator.validate(test_notes.rewrite(data, owner, invalid))


async def test_table_row_properties_pass_schemas_and_invalid_wire_values_fail(validator):
    import test_notes
    table = d.Table(rows=[d.TableRow([d.TableCell([d.Paragraph(text="header")])],
        height=d.RowHeight.exact(14.25), repeat_header=True, cant_split=True),
        d.TableRow([d.TableCell([d.Table(rows=[d.TableRow([d.TableCell([d.Paragraph(text="nested")])],
            height=d.RowHeight.at_least(18), cant_split=False)])])], repeat_header=False)])
    data = await d.Document([d.Section(children=[table], headers={"default": d.Header(children=[table])})]).to_bytes()
    assert {"word/document.xml", "word/header1.xml"} <= set(validator.validate(data))
    for element, attribute, value in [("trHeight", "hRule", "minimum"), ("trHeight", "val", "-1"),
        ("tblHeader", "val", "maybe"), ("cantSplit", "val", "maybe")]:
        def invalid(root):
            root.find(f".//{{{validation.W}}}{element}").set(f"{{{validation.W}}}{attribute}", value)
        with pytest.raises(validation.ValidationError, match="word/document.xml"):
            validator.validate(test_notes.rewrite(data, "word/document.xml", invalid))


async def test_cell_margins_and_directions_pass_schemas_and_bad_values_fail(validator):
    import test_notes
    directions = ["lrTb", "tbRl", "btLr", "lrTbV", "tbRlV", "tbLrV"]
    table = d.Table([d.TableRow([d.TableCell([d.Paragraph(text="cell")], text_direction=direction,
        margins=d.CellMargins(top=3, start=0, bottom=6, end=9)) for direction in directions])])
    data = await d.Document([d.Section(children=[table], headers={"default": d.Header(children=[table])})]).to_bytes()
    assert {"word/document.xml", "word/header1.xml"} <= set(validator.validate(data))
    for element, attribute, value in [("textDirection", "val", "sideways"), ("top", "w", "invalid"), ("top", "type", "points")]:
        def invalid(root):
            root.find(f".//{{{validation.W}}}{element}").set(f"{{{validation.W}}}{attribute}", value)
        with pytest.raises(validation.ValidationError, match="word/document.xml"):
            validator.validate(test_notes.rewrite(data, "word/document.xml", invalid))


async def test_script_formatting_passes_schemas_and_bad_sizes_or_flags_fail(validator):
    import test_notes
    run = d.TextRun("العربية 日本語", language=d.RunLanguage(latin="en-GB", east_asia="ja-JP", bidi="ar-SA"),
        bold_complex_script=True, italic_complex_script=False, size_complex_script=14.5, rtl=True, complex_script=False,
        font="Latin", font_east_asia="East Asian", font_complex_script="Arabic")
    data = await d.Document([d.Section(children=[d.Paragraph(children=[run], style="Script", bidi=True)])],
        styles=[d.ParagraphStyleDefinition(name="Script", language=d.RunLanguage(bidi="he-IL"),
            size_complex_script=13, bold_complex_script=False, italic_complex_script=True, rtl=False, complex_script=True,
            font_east_asia="East Asian", font_complex_script="Hebrew", bidi=False)]).to_bytes()
    assert {"word/document.xml", "word/styles.xml"} <= set(validator.validate(data))
    for owner in ["word/document.xml", "word/styles.xml"]:
        for element, value in [("szCs", "-1"), ("bCs", "maybe"), ("rtl", "maybe"), ("bidi", "maybe")]:
            def invalid(root):
                root.find(f".//{{{validation.W}}}{element}").set(f"{{{validation.W}}}val", value)
            with pytest.raises(validation.ValidationError, match=owner):
                validator.validate(test_notes.rewrite(data, owner, invalid))


async def test_forward_style_references_produce_schema_valid_links(validator):
    data = await d.Document([d.Section(children=[d.Paragraph(text="child", style="Child")])], styles=[
        d.ParagraphStyleDefinition(name="Child", base_style="Later", next_paragraph_style="Later"),
        d.ParagraphStyleDefinition(name="Later", next_paragraph_style="Later"),
        d.TableStyleDefinition(name="Child Table", base_style="Later Table"),
        d.TableStyleDefinition(name="Later Table", base_style="Normal Table"),
    ]).to_bytes()
    assert "word/styles.xml" in validator.validate(data)


async def test_character_style_resources_and_references_are_schema_valid(validator):
    data = await d.Document([d.Section(children=[d.Paragraph(children=[
        d.TextRun("styled", style="Child Character", bold=False),
        d.Hyperlink("https://example.invalid", [d.TextRun("linked", style="Child Character")]),
        d.Field(d.FieldInstruction.page(), [d.TextRun("1", style="Child Character")]),
    ])])], styles=[d.CharacterStyleDefinition(name="Child Character", base_style="Later Character", italic=True,
        language=d.RunLanguage(bidi="ar-SA"), size_complex_script=14.5),
        d.CharacterStyleDefinition(name="Later Character", bold=True)]).to_bytes()
    assert {"word/document.xml", "word/styles.xml"} <= set(validator.validate(data))


async def test_concrete_start_overrides_pass_schemas_and_bad_start_fails(validator):
    import test_notes
    data = await d.Document([d.Section(children=[d.Paragraph(text="restart", numbering=d.NumberingReference(42))])],
        numbering=[d.NumberingDefinition(42, [d.NumberingLevel()], start_overrides=[d.NumberingStartOverride(0, 7)])]).to_bytes()
    assert "word/numbering.xml" in validator.validate(data)
    def invalid(root):
        root.find(f".//{{{validation.W}}}startOverride").set(f"{{{validation.W}}}val", "not-a-number")
    with pytest.raises(validation.ValidationError, match="word/numbering.xml"):
        validator.validate(test_notes.rewrite(data, "word/numbering.xml", invalid))


async def test_concrete_level_replacements_pass_schema_and_invalid_content_fails(validator):
    import test_notes
    data = await d.Document([], numbering=[d.NumberingDefinition(42, [d.NumberingLevel()],
        start_overrides=[d.NumberingStartOverride(0, 7)],
        level_overrides=[d.NumberingLevel(0, 4, "upperRoman", "(%1)")])]).to_bytes()
    assert "word/numbering.xml" in validator.validate(data)
    def invalid_order(root):
        override = root.find(f".//{{{validation.W}}}lvlOverride")
        override[:] = list(reversed(override))
    def invalid_format(root):
        root.find(f".//{{{validation.W}}}lvlOverride/{{{validation.W}}}lvl/{{{validation.W}}}numFmt").set(f"{{{validation.W}}}val", "not-a-format")
    def missing_level_identity(root):
        del root.find(f".//{{{validation.W}}}lvlOverride/{{{validation.W}}}lvl").attrib[f"{{{validation.W}}}ilvl"]
    for mutate in [invalid_order, invalid_format, missing_level_identity]:
        with pytest.raises(validation.ValidationError, match="word/numbering.xml"):
            validator.validate(test_notes.rewrite(data, "word/numbering.xml", mutate))


async def test_table_alignment_passes_schema_and_invalid_token_fails(validator):
    import test_notes
    def table(token):
        return d.Table([d.TableRow([d.TableCell([d.Paragraph(text="alignment")])])], alignment=token)
    data = await d.Document([d.Section([table(token) for token in ["left", "center", "right", "start", "end"]],
        headers={"default": d.Header([table("end")])})]).to_bytes()
    assert "word/document.xml" in validator.validate(data)
    def invalid(root):
        root.find(f".//{{{validation.W}}}tblPr/{{{validation.W}}}jc").set(f"{{{validation.W}}}val", "both")
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(test_notes.rewrite(data, "word/document.xml", invalid))


async def test_numbering_alignment_passes_schema_and_invalid_token_fails(validator):
    import test_notes
    tokens = ["left", "center", "right", "start", "end"]
    data = await d.Document([], numbering=[d.NumberingDefinition(80,
        levels=[d.NumberingLevel(level=i, is_legal=True, suffix="space", restart=0, alignment=token) for i, token in enumerate(tokens)],
        level_overrides=[d.NumberingLevel(level=i, alignment=token) for i, token in enumerate(reversed(tokens))])]).to_bytes()
    assert "word/numbering.xml" in validator.validate(data)
    def invalid(root):
        root.find(f".//{{{validation.W}}}lvlJc").set(f"{{{validation.W}}}val", "invalid")
    with pytest.raises(validation.ValidationError, match="word/numbering.xml"):
        validator.validate(test_notes.rewrite(data, "word/numbering.xml", invalid))


async def test_numbering_restart_passes_schema_and_invalid_integer_fails(validator):
    import test_notes
    values = [0, 1, 2, 4294967295]
    data = await d.Document([], numbering=[d.NumberingDefinition(80,
        levels=[d.NumberingLevel(level=i, is_legal=True, suffix="space", restart=value) for i, value in enumerate(values)],
        level_overrides=[d.NumberingLevel(level=i, restart=value) for i, value in enumerate(reversed(values))])]).to_bytes()
    assert "word/numbering.xml" in validator.validate(data)
    def invalid(root):
        root.find(f".//{{{validation.W}}}lvlRestart").set(f"{{{validation.W}}}val", "one")
    with pytest.raises(validation.ValidationError, match="word/numbering.xml"):
        validator.validate(test_notes.rewrite(data, "word/numbering.xml", invalid))


async def test_numbering_suffix_passes_schema_and_invalid_token_fails(validator):
    import test_notes
    tokens = ["tab", "space", "nothing"]
    data = await d.Document([], numbering=[d.NumberingDefinition(80,
        levels=[d.NumberingLevel(level=i, is_legal=True, suffix=token) for i, token in enumerate(tokens)],
        level_overrides=[d.NumberingLevel(level=i, suffix=token) for i, token in enumerate(reversed(tokens))])]).to_bytes()
    assert "word/numbering.xml" in validator.validate(data)
    def invalid(root):
        root.find(f".//{{{validation.W}}}suff").set(f"{{{validation.W}}}val", "none")
    with pytest.raises(validation.ValidationError, match="word/numbering.xml"):
        validator.validate(test_notes.rewrite(data, "word/numbering.xml", invalid))


async def test_legal_numbering_passes_schema_and_invalid_boolean_fails(validator):
    import test_notes
    data = await d.Document([d.Section([])], numbering=[d.NumberingDefinition(80,
        levels=[d.NumberingLevel(is_legal=True)],
        level_overrides=[d.NumberingLevel(is_legal=False)])]).to_bytes()
    assert "word/numbering.xml" in validator.validate(data)
    def invalid(root):
        root.find(f".//{{{validation.W}}}isLgl").set(f"{{{validation.W}}}val", "maybe")
    with pytest.raises(validation.ValidationError, match="word/numbering.xml"):
        validator.validate(test_notes.rewrite(data, "word/numbering.xml", invalid))


async def test_numbering_uses_the_shared_native_domain(validator):
    document = d.Document([d.Section(children=[d.Paragraph(text="numbered", numbering=d.NumberingReference(42))])],
        numbering=[d.NumberingDefinition(42, [d.NumberingLevel(start=3)])])
    data = await document.to_bytes()
    assert "word/numbering.xml" in validator.validate(data)
    with ZipFile(io.BytesIO(data)) as package:
        root = etree.fromstring(package.read("word/document.xml"))
        assert root.find(f".//{{{validation.W}}}numId").get(f"{{{validation.W}}}val") == "42"
        root = etree.fromstring(package.read("word/numbering.xml"))
        assert root.find(f"{{{validation.W}}}num[@{{{validation.W}}}numId='42']") is not None


async def test_simple_and_nested_complex_fields_pass_schemas_and_balance_checks(validator):
    import test_fields
    import test_notes
    data = await test_fields.declaration().to_bytes()
    assert "word/document.xml" in validator.validate(data)
    image = ROOT / "tests/fixtures/images/sample.png"
    for form in ["simple", "complex"]:
        cached = d.Field(d.FieldInstruction.raw(' PAGE \\* MERGEFORMAT '), [
            d.TextRun("cached", bold=True), d.Tab(), d.Break(), d.ImageRun(data=image),
            d.Hyperlink("https://example.invalid/field", [d.TextRun("link")]),
        ], form=form, dirty=False, locked=True)
        rich = d.Document([d.Section(children=[d.Paragraph(children=[
            d.NoteReference.footnote(0), d.RangeStart.structured_comment("review", d.Comment([d.Paragraph(children=[cached])])),
            d.TextRun("body"), d.RangeEnd("review"),
        ])], headers={"default": d.Header(children=[d.Paragraph(children=[cached])])})],
            notes=[d.Note.footnote(0, [d.Paragraph(children=[cached])])])
        assert {"word/header1.xml", "word/footnotes.xml", "word/comments.xml"} <= set(validator.validate(await rich.to_bytes()))
    def unclosed(root):
        ends = [n for n in root.iter(f"{{{validation.W}}}fldChar") if n.get(f"{{{validation.W}}}fldCharType") == "end"]
        ends[-1].getparent().remove(ends[-1])
    def duplicate_separator(root):
        marks = list(root.iter(f"{{{validation.W}}}fldChar"))
        marks[2].set(f"{{{validation.W}}}fldCharType", "separate")
    for mutation in [unclosed, duplicate_separator]:
        with pytest.raises(validation.ValidationError, match="field"):
            validator.validate(test_notes.rewrite(data, "word/document.xml", mutation))


async def test_external_hyperlinks_and_display_runs_pass_official_content_models(validator):
    def paragraph(with_comment=False):
        return d.Paragraph(children=[d.Hyperlink("https://example.invalid/λ?q=a%20b&x=2#frag", [
            d.TextRun("link", bold=True, comment_text="link note" if with_comment else None), d.Tab(), d.Break(),
            d.ImageRun(data=ROOT / "tests/fixtures/images/sample.png")], tooltip="A <&> λ", history=True)])
    data = await d.Document([d.Section(children=[paragraph(True), d.Table(rows=[d.TableRow(children=[
        d.TableCell(children=[paragraph()])])])], headers={"default": d.Header(children=[paragraph()])},
        footers={"default": d.Footer(children=[paragraph()])})]).to_bytes()
    covered = validator.validate(data)
    assert "word/document.xml" in covered
    assert "word/header1.xml" in covered
    assert "word/footer1.xml" in covered
    output = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source, ZipFile(output, "w", ZIP_DEFLATED) as target:
        for name in source.namelist():
            payload = source.read(name)
            if name == "word/document.xml":
                root = etree.fromstring(payload)
                link = root.find(f".//{{{validation.W}}}hyperlink")
                link.append(etree.Element(f"{{{validation.W}}}p"))
                payload = etree.tostring(root)
            target.writestr(name, payload)
    with pytest.raises(validation.ValidationError, match="word/document.xml"):
        validator.validate(output.getvalue())


async def test_overlapping_bookmarks_cross_paragraph_comments_and_internal_links_pass_schemas(validator):
    data = await d.Document([d.Section(children=[
        d.Paragraph(children=[d.Hyperlink(d.BookmarkTarget("beta"), [d.TextRun("jump")]),
            d.RangeStart.bookmark("alpha"), d.TextRun("A"), d.RangeStart.comment("review", "note", "Ada")]),
        d.Paragraph(children=[d.RangeStart.bookmark("beta"), d.TextRun("B"),d.RangeEnd.bookmark("alpha"),
            d.TextRun("C"),d.RangeEnd("review"),d.RangeEnd.bookmark("beta")]),
        d.Table(rows=[d.TableRow(children=[
            d.TableCell(children=[d.Paragraph(children=[d.RangeStart.bookmark("table"),d.TextRun("first")])]),
            d.TableCell(children=[d.Paragraph(children=[d.TextRun("second"),d.RangeEnd.bookmark("table")])])])])],
        headers={"default":d.Header(children=[d.Paragraph(children=[d.RangeStart.bookmark("header"),
            d.TextRun("header"),d.RangeEnd.bookmark("header")])])})]).to_bytes()
    covered = validator.validate(data)
    assert "word/document.xml" in covered
    assert "word/comments.xml" in covered
    assert "word/header1.xml" in covered


async def test_notes_and_structured_comments_pass_schemas_and_semantic_graph_checks(validator):
    import copy
    import test_notes
    data=await test_notes.declaration().to_bytes()
    covered=validator.validate(data)
    assert {"word/footnotes.xml","word/endnotes.xml","word/comments.xml"} <= set(covered)
    assert "word/footnotes.xml" in validator.validate(test_notes.large_id_package(data))
    extra=io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source,ZipFile(extra,"w",ZIP_DEFLATED) as target:
        for name in source.namelist():
            payload=source.read(name)
            if name=="word/_rels/document.xml.rels":
                root=etree.fromstring(payload)
                etree.SubElement(root,f"{{{validation.REL}}}Relationship",Id="customRelatedNotes",Type="urn:custom/footnotes",Target="custom.xml")
                payload=etree.tostring(root)
            target.writestr(name,payload)
        target.writestr("word/custom.xml",b"<extension/>")
    assert "word/footnotes.xml" in validator.validate(extra.getvalue())
    def missing(root):
        root.remove(next(n for n in root if n.get(f"{{{validation.W}}}id")=="0"))
    def duplicate(root):
        note=copy.deepcopy(next(n for n in root if n.get(f"{{{validation.W}}}id")=="0"))
        note.set(f"{{{validation.W}}}id","+0")
        root.append(note)
    def special(root):
        next(n for n in root if n.get(f"{{{validation.W}}}id")=="0").set(f"{{{validation.W}}}type","separator")
    def nested(root):
        note=next(n for n in root if n.get(f"{{{validation.W}}}id")=="0")
        run=etree.SubElement(note.find(f"{{{validation.W}}}p"),f"{{{validation.W}}}r")
        etree.SubElement(run,f"{{{validation.W}}}endnoteReference").set(f"{{{validation.W}}}id","0")
    for change in [missing,duplicate,special,nested]:
        mutated=test_notes.rewrite(data,"word/footnotes.xml",change)
        with pytest.raises(validation.ValidationError,match="note"):
            validator.validate(mutated)
    def ordinary_settings_target(root):
        root.find(f"{{{validation.W}}}footnotePr/{{{validation.W}}}footnote").set(f"{{{validation.W}}}id","0")
    with pytest.raises(validation.ValidationError,match="special"):
        validator.validate(test_notes.rewrite(data,"word/settings.xml",ordinary_settings_target))
