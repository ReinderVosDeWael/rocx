"""Immutable resolved values passed directly to the native domain adapter."""
import dataclasses
from collections.abc import Mapping


@dataclasses.dataclass(frozen=True)
class ResolvedNode:
    _native_kind: str
    _node_id: int
    _fields: Mapping

    def __getattr__(self, name):
        try:
            return self._fields[name]
        except KeyError:
            raise AttributeError(name) from None

    async def to_bytes(self, template=None, *, options=None):
        if self._native_kind != "Document":
            raise TypeError("Only a resolved Document can be rendered")
        if template is not None and not isinstance(template, ResolvedNode):
            from . import resolve_component
            template = await resolve_component(template)
        from rocx import _native
        if options is None:
            return _native.render(self, template)
        return _native.render(self, template, options)

    async def to_docx(self, template=None, *, options=None):
        from rocx.interop import to_docx
        return to_docx(await self.to_bytes(template, options=options))

    async def save(self, path, template=None, *, options=None):
        from rocx.io import save_bytes
        save_bytes(path, await self.to_bytes(template, options=options))
