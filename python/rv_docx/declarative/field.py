"""Field instructions and cached results; never a field evaluation engine."""
from __future__ import annotations

import dataclasses

from rv_docx.declarative import base


@dataclasses.dataclass
class FieldInstruction(base.Component):
    """Typed common instructions or an explicit unparsed escape hatch.

    Attributes:
        kind: page, num_pages, reference, table_of_contents, or raw.
        name: Bookmark name for a reference; checked against generated/imported names.
        code: Raw instruction text, preserved verbatim without interpreting targets.
    """

    kind: str
    name: str | None = None
    code: str | None = None

    @classmethod
    def page(cls) -> FieldInstruction:
        """Request a PAGE instruction, not a calculated page number."""
        return cls("page")

    @classmethod
    def num_pages(cls) -> FieldInstruction:
        """Request NUMPAGES without calculating pagination."""
        return cls("num_pages")

    @classmethod
    def reference(cls, name: str) -> FieldInstruction:
        """Request a REF to a checked bookmark name."""
        return cls("reference", name=name)

    @classmethod
    def table_of_contents(cls) -> FieldInstruction:
        """Request a heading-level 1–3 TOC; cached content remains caller-supplied."""
        return cls("table_of_contents")

    @classmethod
    def raw(cls, code: str) -> FieldInstruction:
        """Preserve nonempty XML instruction text without parsing field syntax."""
        return cls("raw", code=code)


@dataclasses.dataclass
class Field(base.Component):
    """A paragraph-local balanced field with independently deferred properties.

    Attributes:
        instruction: FieldInstruction, optionally deferred.
        children: Cached rich inlines, optionally deferred. Nested fields are legal;
            range markers and fields inside hyperlinks are unsupported.
        form: simple (default) or complex begin/instruction/separate/result/end.
        dirty: Optional consumer-update request, not library-side evaluation.
        locked: Optional field lock; absent/false/true remain distinct.
    """

    instruction: FieldInstruction
    children: object = dataclasses.field(default_factory=list)
    form: str = "simple"
    dirty: bool | None = None
    locked: bool | None = None
