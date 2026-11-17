"""Python-native declaration and asyncio resolution; OOXML rendering is Rust."""
import asyncio
from collections.abc import Iterator
import contextvars
import dataclasses
import inspect
from typing import Generic, TypeVar

_resolution = contextvars.ContextVar("rv_docx_resolution", default=None)
_ancestors = contextvars.ContextVar("rv_docx_ancestors", default=frozenset())
_current = contextvars.ContextVar("rv_docx_current_property", default=None)
_COMPONENT = object()
_CONDITION = object()
_T = TypeVar("_T")


class _Resolution:
    """Share field work and track waits across concurrently running branches."""

    def __init__(self):
        self.tasks = {}
        self.waits = {}
        self.owners = {}  # Keep identity keys valid until this pass completes.

    def _reaches(self, start, target):
        pending = [start]
        visited = set()
        while pending:
            key = pending.pop()
            if key == target:
                return True
            if key not in visited:
                visited.add(key)
                pending.extend(self.waits.get(key, ()))
        return False

    async def run(self, key, work, label):
        caller = _current.get()
        if caller is not None:
            if self._reaches(key, caller):
                raise ValueError(f"Cyclic deferred property dependency: {label}")
            edges = self.waits.setdefault(caller, {})
            edges[key] = edges.get(key, 0) + 1
        try:
            task = self.tasks.get(key)
            if task is None:
                async def execute():
                    token = _current.set(key)
                    try:
                        return await work()
                    finally:
                        _current.reset(token)
                task = asyncio.create_task(execute())
                self.tasks[key] = task
            return await task
        finally:
            if caller is not None:
                edges = self.waits[caller]
                edges[key] -= 1
                if not edges[key]:
                    del edges[key]


async def _gather(values):
    tasks = [asyncio.ensure_future(value) for value in values]
    try:
        return await asyncio.gather(*tasks)
    except BaseException:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
        raise


def _field_names(owner):
    if dataclasses.is_dataclass(owner) and not isinstance(owner, type):
        return [field.name for field in dataclasses.fields(owner) if field.name != "condition"]
    return getattr(type(owner), "_deferred_fields", ())


async def _condition(owner):
    context = _resolution.get()

    async def evaluate():
        condition = owner.condition() if callable(owner.condition) else owner.condition
        if inspect.isawaitable(condition):
            if inspect.iscoroutine(condition):
                condition.close()
            raise TypeError("Component condition must be synchronous")
        owner._active = bool(condition)
        return owner._active

    return await context.run((id(owner), _CONDITION), evaluate, f"{type(owner).__name__}.condition")


async def _resolve_property(owner, name):
    context = _resolution.get()
    token = None
    if context is None:
        context = _Resolution()
        token = _resolution.set(context)
    try:
        context.owners[id(owner)] = owner
        if isinstance(owner, Component) and not await _condition(owner):
            raise ValueError(f"Cannot resolve deferred property '{name}' on excluded {type(owner).__name__}")

        async def evaluate():
            result = await resolve_value(getattr(owner, name))
            setattr(owner, name, result)
            return result

        return await context.run((id(owner), name), evaluate, f"{type(owner).__name__}.{name}")
    finally:
        if token is not None:
            _resolution.reset(token)


class DeferredProperty(Generic[_T]):
    """An awaitable reference to one declaration field, resolved on demand.

    Accepts any Component, Document, or declaration dataclass field. It waits
    for that field alone, so independent properties can depend on each other
    without waiting for their entire components. A resolution pass shares the
    producer and propagates its original exception to all consumers.
    """

    def __init__(self, owner, name: str):
        if name not in _field_names(owner):
            raise AttributeError(f"{type(owner).__name__} has no resolvable property {name!r}")
        self.owner = owner
        self.name = name

    def __await__(self):
        return self.resolve().__await__()

    async def resolve(self) -> _T:
        return await _resolve_property(self.owner, self.name)


async def resolve_value(value):
    while True:
        if isinstance(value, Component):
            return await value.resolve()
        if inspect.isawaitable(value):
            resolved = await value
            if resolved is value:
                raise ValueError("An awaitable field cannot resolve to itself")
            value = resolved
        elif callable(value):
            value = value()
        else:
            break
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        await _gather(_resolve_property(value, name) for name in _field_names(value))
    elif isinstance(value, dict):
        keys = list(value)
        results = await _gather(resolve_value(value[key]) for key in keys)
        value = dict(zip(keys, results, strict=True))
    elif isinstance(value, (list, tuple, Iterator)):
        results = await _gather(resolve_value(item) for item in value)
        value = tuple(results) if isinstance(value, tuple) else results
    return value


@dataclasses.dataclass
class Component:
    condition: object = dataclasses.field(default=lambda: True, kw_only=True)

    def deferred(self, name: str) -> DeferredProperty:
        """Refer to a field without evaluating its lazy or async value yet."""
        return DeferredProperty(self, name)

    def __await__(self):
        return self.resolve().__await__()

    async def resolve(self):
        if id(self) in _ancestors.get():
            raise ValueError("A document component cannot contain itself")
        context = _resolution.get()
        token = None
        if context is None:
            context = _Resolution()
            token = _resolution.set(context)
        try:
            context.owners[id(self)] = self
            return await context.run((id(self), _COMPONENT), self._resolve_fields, type(self).__name__)
        finally:
            if token is not None:
                _resolution.reset(token)

    async def _resolve_fields(self):
        if not await _condition(self):
            return self
        token = _ancestors.set(_ancestors.get() | {id(self)})
        try:
            await _gather(_resolve_property(self, name) for name in _field_names(self))
            return self
        finally:
            _ancestors.reset(token)
