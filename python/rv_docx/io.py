"""Filesystem effects invoked by resolution and application orchestration."""
import os
from pathlib import Path
import tempfile
from . import diagnostics


def save_bytes(destination, data):
    if hasattr(destination, "write"):
        try:
            destination.write(data)
        except OSError as error:
            diagnostics.annotate(error, "io_error", "output")
            raise
        return
    destination = Path(destination)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=destination.parent, prefix=".rv-docx-", delete=False) as output:
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
