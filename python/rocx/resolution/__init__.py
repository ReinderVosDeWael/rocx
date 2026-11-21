"""Async authoring interpreter. Resolved snapshots never mutate declarations."""
import asyncio
from collections.abc import Iterator, Mapping
import contextvars
import dataclasses
import inspect
import itertools
from types import MappingProxyType
from typing import Generic, TypeVar

from .snapshot import ResolvedNode
from .. import diagnostics

_ids = itertools.count(1)
_context = contextvars.ContextVar("rocx_resolution", default=None)
_current = contextvars.ContextVar("rocx_property", default=None)
_OMIT = object()
_COMPONENT = object()
_CONDITION = object()
_T = TypeVar("_T")


def identity(owner):
    node_id = getattr(owner, "_node_id", None)
    if node_id is None:
        node_id = next(_ids)
        object.__setattr__(owner, "_node_id", node_id)
    return node_id


def field_names(owner):
    if dataclasses.is_dataclass(owner) and not isinstance(owner, type):
        return [field.name for field in dataclasses.fields(owner) if field.init and field.name != "condition"]
    return getattr(type(owner), "_deferred_fields", ())


class DependencyError(ValueError):
    """A dependency rule failure, with a stable code for language adapters."""
    def __init__(self, rule, message, location=None):
        self.rule = rule
        self.location = location
        self.cause = None
        super().__init__(message)


class _Resolution:
    def __init__(self):
        self.tasks = {}
        self.waits = {}
        self.owners = {}
        self.asset_names = {}

    def reaches(self, start, target):
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
            if self.reaches(key, caller):
                raise DependencyError("dependency_cycle", f"Cyclic deferred property dependency: {label}", label)
            edges = self.waits.setdefault(caller, {})
            edges[key] = edges.get(key, 0) + 1
        try:
            task = self.tasks.get(key)
            if task is None:
                async def execute():
                    token = _current.set(key)
                    try:
                        return await work()
                    except Exception as error:
                        # Preserve producer exception identity/categories; attach context
                        # rather than guessing a rule from its human-readable message.
                        diagnostics.annotate(error, "invalid_content" if key[1] is _CONDITION else "producer_error", label)
                        raise
                    finally:
                        _current.reset(token)
                task = asyncio.create_task(execute())
                self.tasks[key] = task
            # The pass owns the task; an individual cancelled consumer cannot
            # cancel a producer another consumer is still using.
            return await asyncio.shield(task)
        finally:
            if caller is not None:
                edges = self.waits[caller]
                edges[key] -= 1
                if not edges[key]:
                    del edges[key]

    async def close(self):
        tasks = list(self.tasks.values())
        for task in tasks:
            if not task.done():
                task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)


async def _pass(work):
    if _context.get() is not None:
        return await work()
    context = _Resolution()
    token = _context.set(context)
    try:
        return await work()
    finally:
        await context.close()
        _context.reset(token)


async def gather(values):
    tasks = [asyncio.ensure_future(value) for value in values]
    try:
        return await asyncio.gather(*tasks)
    except BaseException:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
        raise


async def _condition(owner):
    if not hasattr(owner, "condition"):
        return True
    async def evaluate():
        condition = owner.condition() if callable(owner.condition) else owner.condition
        if inspect.isawaitable(condition):
            if inspect.iscoroutine(condition):
                condition.close()
            raise TypeError("Component condition must be synchronous")
        return bool(condition)
    return await _context.get().run((identity(owner), _CONDITION), evaluate, f"{type(owner).__name__}.condition")


async def _property(owner, name):
    context = _context.get()
    context.owners[identity(owner)] = owner
    if not await _condition(owner):
        raise DependencyError("excluded_dependency", f"Cannot resolve deferred property '{name}' on excluded {type(owner).__name__}", f"{type(owner).__name__}.{name}")
    async def evaluate():
        value = await resolve_value(getattr(owner, name))
        if getattr(owner, "_native_kind", None) == "ImageRun" and name == "data":
            from rocx.io import image_bytes
            value, filename = image_bytes(value)
            context.asset_names[identity(owner)] = filename
        return value
    return await context.run((identity(owner), name), evaluate, f"{type(owner).__name__}.{name}")


async def resolve_property(owner, name):
    return await _pass(lambda: _property(owner, name))


async def _component(owner):
    context = _context.get()
    context.owners[identity(owner)] = owner
    async def evaluate():
        if not await _condition(owner):
            return _OMIT
        names = field_names(owner)
        values = await gather(_property(owner, name) for name in names)
        fields = dict(zip(names, values, strict=True))
        kind = getattr(owner, "_native_kind", type(owner).__name__)
        if kind == "ImageRun":
            fields["_filename"] = context.asset_names.get(identity(owner))
        if kind == "Document" and fields.get("timestamp") is None:
            from rocx.io import timestamp
            fields["timestamp"] = timestamp()
        return ResolvedNode(kind, identity(owner), MappingProxyType(fields))
    return await context.run((identity(owner), _COMPONENT), evaluate, type(owner).__name__)


async def resolve_component(owner):
    result = await _pass(lambda: _component(owner))
    return None if result is _OMIT else result


async def resolve_value(value):
    from rocx.declarative.base import Component
    while True:
        if isinstance(value, ResolvedNode):
            return value
        if isinstance(value, Component) or hasattr(type(value), "_deferred_fields"):
            return await _component(value)
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
        return await _component(value)
    if isinstance(value, Mapping):
        keys = list(value)
        values = await gather(resolve_value(value[key]) for key in keys)
        return MappingProxyType({key: item for key, item in zip(keys, values, strict=True) if item is not _OMIT})
    if isinstance(value, (list, tuple, Iterator)):
        values = await gather(resolve_value(item) for item in value)
        return tuple(item for item in values if item is not _OMIT)
    if isinstance(value, (bytearray, memoryview)):
        return bytes(value)
    from enum import Enum
    from pathlib import PurePath
    if value is None or isinstance(value, (str, bytes, int, float, complex, Enum, PurePath)):
        return value
    raise TypeError(f"Unsupported resolved value type: {type(value).__name__}")


class DeferredProperty(Generic[_T]):
    """A per-pass reference to a declared field, not a memoized computation."""
    def __init__(self, owner, name: str):
        if name not in field_names(owner):
            raise AttributeError(f"{type(owner).__name__} has no resolvable property {name!r}")
        self.owner, self.name = owner, name
        identity(owner)

    def __await__(self):
        return self.resolve().__await__()

    async def resolve(self) -> _T:
        return await resolve_property(self.owner, self.name)


PropertyRef = DeferredProperty


class Deferred(Generic[_T]):
    """A lazy computation memoized for this handle's lifetime.

    Awaiters share the producer; cancelling one awaiter leaves the others intact.
    Use cancel() to explicitly cancel this externally owned computation.
    """
    def __init__(self, source):
        self._source = source
        self._task = None
        self._complete = False
        self._result = None
        self._error = None

    def __await__(self):
        return self.resolve().__await__()

    async def resolve(self) -> _T:
        if not self._complete:
            if self._task is None:
                async def run():
                    try:
                        self._result = await _pass(lambda: resolve_value(self._source))
                    except BaseException as error:
                        self._error = error
                    finally:
                        self._complete = True
                # An explicitly shared computation owns an independent pass.
                context = contextvars.copy_context()
                context.run(_context.set, None)
                context.run(_current.set, None)
                self._task = asyncio.create_task(run(), context=context)
            await asyncio.shield(self._task)
        if self._error is not None:
            raise self._error
        return self._result

    def cancel(self):
        if self._task is not None and not self._task.done():
            self._task.cancel()
