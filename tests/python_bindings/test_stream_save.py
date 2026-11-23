import io

import pytest

from rocx import declarative as d


@pytest.fixture
def document():
    return d.Document(
        [d.Section(children=[d.Paragraph(text="Save the entire DOCX")])],
        timestamp="2026-11-18T00:00:00Z",
    )


async def test_save_retries_short_writes_without_changing_docx_bytes(document):
    class ShortWriter:
        def __init__(self):
            self.output = io.BytesIO()

        def write(self, data):
            assert isinstance(data, bytes)
            return self.output.write(data[:97])

    destination = ShortWriter()
    expected = await document.to_bytes()
    await document.save(destination)
    assert destination.output.getvalue() == expected


@pytest.mark.parametrize("result", [0, None, -1, True, "1", 10**10])
async def test_save_rejects_a_stalled_or_invalid_write_after_progress(document, result):
    class StalledWriter:
        def __init__(self):
            self.output = io.BytesIO()
            self.stalled = False

        def write(self, data):
            if not self.output.tell():
                return self.output.write(data[:97])
            assert not self.stalled, "save retried without progress"
            self.stalled = True
            return result

    destination = StalledWriter()
    expected = await document.to_bytes()
    with pytest.raises(OSError) as error:
        await document.save(destination)
    assert error.value.rule == "io_error"
    assert error.value.location == "output"
    assert destination.output.getvalue() == expected[:97]
    assert destination.stalled


async def test_save_preserves_stream_write_errors(document):
    failure = BlockingIOError("stream would block")

    class BlockedWriter:
        def write(self, data):
            raise failure

    with pytest.raises(BlockingIOError) as error:
        await document.save(BlockedWriter())
    assert error.value is failure
    assert error.value.rule == "io_error"
    assert error.value.location == "output"
