"""Put a line-break clear attribute in a relationship-selected main part."""

from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

here = Path(__file__).parent
main = "word/document.xml"
relocated = "word/flow.xml"

with (
    ZipFile(here / "physical-plain.docx") as source,
    ZipFile(here / "physical-clearing-break.docx", "w", ZIP_DEFLATED) as target,
):
    document = source.read(main)
    assert document.count(b"<w:br/>") == 1
    for part in source.namelist():
        payload = source.read(part)
        if part in {"[Content_Types].xml", "_rels/.rels"}:
            assert payload.count(main.encode()) == 1
            payload = payload.replace(main.encode(), relocated.encode())
        target.writestr(part, payload)
    # The unmodified original path remains as a decoy.
    target.writestr(relocated, document.replace(b"<w:br/>", b'<w:br w:clear="all"/>'))
    target.writestr("word/_rels/flow.xml.rels", source.read("word/_rels/document.xml.rels"))
