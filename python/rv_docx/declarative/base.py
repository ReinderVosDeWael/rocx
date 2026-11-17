"""Python-native declaration and asyncio resolution; OOXML rendering is Rust."""
import asyncio
from collections.abc import Iterator
import contextvars
import dataclasses
import inspect

_resolution = contextvars.ContextVar("rv_docx_resolution", default=None)
_ancestors = contextvars.ContextVar("rv_docx_ancestors", default=frozenset())

async def _gather(values):
    tasks = [asyncio.ensure_future(value) for value in values]
    try:
        return await asyncio.gather(*tasks)
    except BaseException:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
        raise

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
        fields = dataclasses.fields(value)
        results = await _gather(resolve_value(getattr(value, field.name)) for field in fields)
        for field, result in zip(fields, results, strict=True):
            setattr(value, field.name, result)
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

    def __await__(self):
        return self.resolve().__await__()

    async def resolve(self):
        if id(self) in _ancestors.get():
            raise ValueError("A document component cannot contain itself")
        context = _resolution.get()
        token = None
        if context is None:
            context = {}
            token = _resolution.set(context)
        try:
            task = context.get(id(self))
            if task is None:
                task = asyncio.create_task(self._resolve_fields())
                context[id(self)] = task
            return await task
        finally:
            if token is not None:
                _resolution.reset(token)

    async def _resolve_fields(self):
        condition = self.condition() if callable(self.condition) else self.condition
        if inspect.isawaitable(condition):
            if inspect.iscoroutine(condition):
                condition.close()
            raise TypeError("Component condition must be synchronous")
        self._active = bool(condition)
        if not self._active:
            return self
        token = _ancestors.set(_ancestors.get() | {id(self)})
        try:
            fields = [field for field in dataclasses.fields(self) if field.name != "condition"]
            results = await _gather(resolve_value(getattr(self, field.name)) for field in fields)
            for field, result in zip(fields, results, strict=True):
                setattr(self, field.name, result)
            return self
        finally:
            _ancestors.reset(token)
