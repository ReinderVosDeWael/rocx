"""Declarative document entrypoints backed by immutable snapshots and Rust."""
import dataclasses

from .base import DeferredProperty
from rocx.resolution import identity, resolve_component

@dataclasses.dataclass
class DocumentTemplate:
    path: object
    replacements: dict[str, str] | None = None
    paragraph_index: int | None = None
    scope: str = "body_headers_footers"

class Document:
    _deferred_fields = (
        "sections", "creator", "title", "subject", "description", "keywords",
        "category", "comments", "version", "comment_author", "styles", "numbering", "timestamp", "notes", "defaults",
    )

    def deferred(self, name: str) -> DeferredProperty:
        return DeferredProperty(self, name)

    def __init__(self, sections, creator=None, title=None, subject=None,
                 description=None, keywords=None, category=None, comments=None,
                 version=None, comment_author=None, styles=None, numbering=None, timestamp=None, notes=None, defaults=None):
        identity(self)
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
        self.timestamp = timestamp
        self.notes = notes
        self.defaults = defaults

    def __await__(self):
        return self.resolve().__await__()

    async def resolve(self):
        return await resolve_component(self)

    async def to_bytes(self, template=None, *, options=None):
        return await (await self.resolve()).to_bytes(template, options=options)

    async def to_docx(self, template=None, *, options=None):
        return await (await self.resolve()).to_docx(template, options=options)

    async def save(self, path, template=None, *, options=None):
        return await (await self.resolve()).save(path, template, options=options)
