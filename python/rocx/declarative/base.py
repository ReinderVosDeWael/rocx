"""Authoring declarations; asyncio execution lives in rocx.resolution."""
import dataclasses
from rocx.resolution import Deferred, DeferredProperty, PropertyRef, identity, resolve_component


@dataclasses.dataclass
class Component:
    condition: object = dataclasses.field(default=lambda: True, kw_only=True)
    _node_id: int = dataclasses.field(init=False, repr=False, compare=False)

    def __post_init__(self):
        identity(self)

    def deferred(self, name: str) -> DeferredProperty:
        return DeferredProperty(self, name)

    def __await__(self):
        return self.resolve().__await__()

    async def resolve(self):
        return await resolve_component(self)
