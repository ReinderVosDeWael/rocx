"""Drawing identities remain unique when a Word story has a non-XML name."""

import io
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

import docx

from rocx import declarative as d
from rocx.editor import HeaderImageEdit, ImportedDocument


async def test_python_docx_relocated_header_drawing_ids():
    image_bytes = (Path(__file__).parents[1] / "fixtures/images/sample.png").read_bytes()
    source = docx.Document()
    source.sections[0].header.paragraphs[0].add_run().add_picture(io.BytesIO(image_bytes))
    stream = io.BytesIO()
    source.save(stream)

    relocated = io.BytesIO()
    with ZipFile(io.BytesIO(stream.getvalue())) as original, ZipFile(
        relocated, "w", ZIP_DEFLATED
    ) as package:
        for name in original.namelist():
            data = original.read(name)
            if name in ("[Content_Types].xml", "word/_rels/document.xml.rels"):
                data = data.replace(b"header1.xml", b"header1.part")
            name = name.replace("word/header1.xml", "word/header1.part")
            name = name.replace("word/_rels/header1.xml.rels", "word/_rels/header1.part.rels")
            package.writestr(name, data)

    before = docx.Document(io.BytesIO(relocated.getvalue()))
    original_ids = before.sections[0].header._element.xpath(".//wp:docPr/@id")
    assert original_ids == ["1"]
    imported = ImportedDocument.open(relocated.getvalue())
    image = await d.ImageRun(data=image_bytes).resolve()
    output = imported.preview_header_image(
        HeaderImageEdit(imported.default_headers[0].anchor, image, mode="shared")
    ).to_docx()
    after = docx.Document(io.BytesIO(output))
    ids = after.sections[0].header._element.xpath(".//wp:docPr/@id")
    assert len(ids) == 2
    assert ids[:1] == original_ids
    assert ids[1] != ids[0]
