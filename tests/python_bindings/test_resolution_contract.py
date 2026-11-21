"""R1 scenarios paired with Rust's resolution_contract integration tests."""

import asyncio
import io
import pathlib
import zipfile

import docx
import pytest
from rocx import declarative as d


async def test_image_fields_accept_references_before_the_source_and_snapshots_do_not_rerun_them() -> None:
    calls = []

    def data() -> bytes:
        calls.append("data")
        return (pathlib.Path(__file__).parents[1] / "fixtures/images/sample.png").read_bytes()

    source = d.ImageRun(data=data, transformation=lambda: {"width": 36, "height": 18},
                        alt_text=lambda: {"title": "λ picture"})
    copy = d.ImageRun(data=source.deferred("data"),
                      transformation=source.deferred("transformation"),
                      alt_text=source.deferred("alt_text"))
    snapshot = await d.Document([d.Section(children=[d.Paragraph(children=[copy, source])])]).resolve()
    first = await snapshot.to_bytes()
    assert first == await snapshot.to_bytes()
    document = docx.Document(io.BytesIO(first))
    assert [(shape.width, shape.height) for shape in document.inline_shapes] == [(457200, 228600)] * 2
    assert [shape._inline.docPr.get("title") for shape in document.inline_shapes] == ["λ picture"] * 2
    assert calls == ["data"]
    assert source.data is data


async def test_image_fields_start_concurrently_and_hidden_images_do_not_poll_options() -> None:
    started = {name: asyncio.Event() for name in ("data", "transformation", "alt_text")}
    gate = asyncio.Event()

    async def value(name: str, result: object) -> object:
        started[name].set()
        await gate.wait()
        return result

    data = (pathlib.Path(__file__).parents[1] / "fixtures/images/sample.png").read_bytes()
    image = d.ImageRun(data=lambda: value("data", data),
                       transformation=lambda: value("transformation", None),
                       alt_text=lambda: value("alt_text", None))
    task = asyncio.create_task(image.resolve())
    try:
        await asyncio.wait_for(asyncio.gather(*(event.wait() for event in started.values())), 5)
    finally:
        gate.set()
        await task

    def forbidden() -> None:
        pytest.fail("hidden image producer ran")

    hidden = d.ImageRun(data=forbidden, transformation=forbidden, alt_text=forbidden, condition=False)
    assert await hidden.resolve() is None


@pytest.mark.parametrize("field", ["transformation", "alt_text"])
async def test_image_option_producer_errors_preserve_the_original_cause(field: str) -> None:
    failure = PermissionError("option sentinel")

    def fail() -> None:
        raise failure

    image = d.ImageRun(data=(pathlib.Path(__file__).parents[1] / "fixtures/images/sample.png").read_bytes())
    setattr(image, field, fail)
    with pytest.raises(PermissionError) as error:
        await image.resolve()
    assert error.value is failure


@pytest.mark.parametrize("explicit_share", [False, True])
async def test_concurrent_root_passes_have_deliberate_sharing_lifetimes(explicit_share: bool) -> None:
    calls = []
    both_started = asyncio.Event()
    gate = asyncio.Event()

    async def producer() -> str:
        calls.append("text")
        if len(calls) == (1 if explicit_share else 2):
            both_started.set()
        await gate.wait()
        return "shared λ"

    source = d.Paragraph(text=d.Deferred(producer) if explicit_share else producer)
    first = asyncio.create_task(d.Paragraph(text=source.deferred("text")).resolve())
    second = asyncio.create_task(d.Paragraph(text=source.deferred("text")).resolve())
    try:
        await asyncio.wait_for(both_started.wait(), 5)
    finally:
        gate.set()
        results = await asyncio.gather(first, second)
    assert [result.text for result in results] == ["shared λ"] * 2
    assert calls == ["text"] * (1 if explicit_share else 2)


async def test_nested_passes_keep_cycle_detection_and_source_conditions() -> None:
    source = d.Paragraph(text="placeholder")

    async def nested() -> str:
        return (await source.resolve()).text

    source.text = nested
    with pytest.raises(ValueError) as error:
        await asyncio.wait_for(source.resolve(), 5)
    assert error.value.rule == "dependency_cycle"

    hidden = d.Paragraph(text="hidden", condition=False)
    with pytest.raises(ValueError) as error:
        await hidden.deferred("text")
    assert error.value.rule == "excluded_dependency"


async def test_cancelling_one_field_waiter_keeps_the_other_waiter_usable() -> None:
    started = asyncio.Event()
    gate = asyncio.Event()
    calls = []

    async def producer() -> str:
        calls.append("text")
        started.set()
        await gate.wait()
        return "survivor"

    source = d.Paragraph(text=producer)

    async def consumers() -> str:
        first = asyncio.create_task(source.deferred("text").resolve())
        second = asyncio.create_task(source.deferred("text").resolve())
        try:
            await started.wait()
            first.cancel()
            with pytest.raises(asyncio.CancelledError):
                await first
            gate.set()
            return await second
        finally:
            first.cancel()
            second.cancel()
            await asyncio.gather(first, second, return_exceptions=True)

    snapshot = await asyncio.wait_for(d.Paragraph(text=consumers).resolve(), 5)
    assert snapshot.text == "survivor"
    assert calls == ["text"]


@pytest.mark.parametrize("explicit_share", [False, True])
async def test_cancelling_a_pass_drains_owned_work_but_not_external_computations(explicit_share: bool) -> None:
    started = asyncio.Event()
    stopped = asyncio.Event()
    gate = asyncio.Event()

    async def producer() -> str:
        try:
            started.set()
            await gate.wait()
            return "resumed"
        finally:
            stopped.set()

    shared = d.Deferred(producer)
    task = asyncio.create_task(d.Paragraph(text=shared if explicit_share else producer).resolve())
    try:
        await asyncio.wait_for(started.wait(), 5)
        task.cancel()
        with pytest.raises(asyncio.CancelledError):
            await task
        assert stopped.is_set() is not explicit_share
        if explicit_share:
            gate.set()
            assert await shared == "resumed"
            assert stopped.is_set()
    finally:
        gate.set()
        shared.cancel()
        task.cancel()
        await asyncio.gather(task, return_exceptions=True)


async def test_image_snapshots_survive_source_mutation_and_deletion(tmp_path: pathlib.Path) -> None:
    original = (pathlib.Path(__file__).parents[1] / "fixtures/images/sample.png").read_bytes()
    image = tmp_path / "input.png"
    image.write_bytes(original)
    declaration = d.Document([d.Section(children=[d.Paragraph(children=[d.ImageRun(data=image)])])])
    snapshot = await declaration.resolve()
    first = await snapshot.to_bytes()
    image.write_bytes(b"not an image")
    assert first == await snapshot.to_bytes()
    image.unlink()
    assert first == await snapshot.to_bytes()
    with zipfile.ZipFile(io.BytesIO(first)) as package:
        assert package.read("word/media/image1.png") == original
