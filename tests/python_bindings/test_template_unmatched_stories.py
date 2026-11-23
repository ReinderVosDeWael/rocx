"""Preserve unmatched stories from an independently authored template."""
import io
import zipfile

from docx import Document
from rocx import declarative as d


async def test_unmatched_python_docx_stories_are_byte_preserved(tmp_path):
    source = tmp_path / "independent.docx"
    fixture = Document()
    fixture.add_paragraph("Hello {{NAME}}")
    fixture.sections[0].header.paragraphs[0].text = "Constant header"
    fixture.sections[0].footer.paragraphs[0].text = "Constant footer"
    fixture.save(source)

    result = await d.Document([]).to_bytes(
        d.DocumentTemplate(source, replacements={"{{NAME}}": "world"})
    )
    with zipfile.ZipFile(source) as before, zipfile.ZipFile(io.BytesIO(result)) as after:
        for part in ("word/header1.xml", "word/footer1.xml"):
            assert before.read(part) == after.read(part)
    assert Document(io.BytesIO(result)).paragraphs[0].text == "Hello world"
