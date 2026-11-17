"""CMI-compatible document entry points backed by native Rust rendering."""
import dataclasses
import io
import os
import zipfile

from .base import Component

@dataclasses.dataclass
class DocumentTemplate:
    path: object
    replacements: dict[str, str] | None = None
    paragraph_index: int | None = None

class Document:
    def __init__(self, sections, creator=None, title=None, subject=None,
                 description=None, keywords=None, category=None, comments=None,
                 version=None, comment_author=None, styles=None, numbering=None):
        self.sections = sections
        self.creator = creator
        self.title = title
        self.subject = subject
        self.description = description
        self.keywords = keywords
        self.category = category
        self.comments = comments
        self.version = version
        self.comment_author = comment_author
        self.styles = styles
        self.numbering = numbering

    def __await__(self):
        return self.resolve().__await__()

    async def resolve(self):
        return await Component.resolve(self)

    async def _resolve_fields(self):
        from .base import _gather, resolve_value
        self._active = True
        values = await _gather(resolve_value(value) for value in [self.sections, self.styles, self.numbering])
        self.sections, self.styles, self.numbering = values
        return self

    async def to_bytes(self, template=None):
        await self.resolve()
        if template is not None and not zipfile.is_zipfile(os.fspath(template.path)):
            from docx.opc.exceptions import PackageNotFoundError
            raise PackageNotFoundError(f"Package not found at '{template.path}'")
        from rv_docx import _native
        return _native.render(self, template)

    async def to_docx(self, template=None):
        import docx
        return docx.Document(io.BytesIO(await self.to_bytes(template)))

    async def save(self, path, template=None):
        data = await self.to_bytes(template)
        if hasattr(path, "write"):
            path.write(data)
        else:
            with open(os.fspath(path), "wb") as destination:
                destination.write(data)
