"""Filesystem effects invoked by resolution and application orchestration."""
from io import DEFAULT_BUFFER_SIZE
import os
from pathlib import Path
import tempfile
from . import diagnostics


def save_bytes(destination, data):
    if hasattr(destination, "write"):
        try:
            offset = 0
            chunk = data
            while chunk:
                written = destination.write(chunk)
                if written is None:
                    raise OSError("output stream write made no progress")
                if (
                    not isinstance(written, int)
                    or isinstance(written, bool)
                    or not 0 <= written <= len(chunk)
                ):
                    raise OSError("output stream write returned an invalid byte count")
                if written == 0:
                    raise OSError("output stream write made no progress")
                offset += written
                # Keep passing bytes to custom writers, without repeatedly
                # copying the entire unwritten suffix after a short write.
                chunk = data[offset:offset + DEFAULT_BUFFER_SIZE]
        except OSError as error:
            diagnostics.annotate(error, "io_error", "output")
            raise
        return
    destination = Path(destination)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=destination.parent, prefix=".rocx-", delete=False) as output:
            temporary = output.name
            output.write(data)
            output.flush()
        os.replace(temporary, destination)
    except OSError as error:
        diagnostics.annotate(error, "io_error", "output")
        raise
    finally:
        if temporary is not None and os.path.exists(temporary):
            os.unlink(temporary)


def timestamp():
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def image_bytes(source):
    if isinstance(source, bytes):
        return source, None
    if isinstance(source, (str, os.PathLike)):
        path = Path(source)
        try:
            return path.read_bytes(), path.name
        except OSError as error:
            diagnostics.annotate(error, "io_error", "ImageRun.data")
            raise
    raise TypeError("ImageRun.data must resolve to bytes or a filesystem path")
