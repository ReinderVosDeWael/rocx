"""Filesystem effects invoked by resolution and application orchestration."""
import os
from pathlib import Path
import tempfile


def save_bytes(destination, data):
    if hasattr(destination, "write"):
        destination.write(data)
        return
    destination = Path(destination)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=destination.parent, prefix=".rv-docx-", delete=False) as output:
            temporary = output.name
            output.write(data)
            output.flush()
        os.replace(temporary, destination)
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
        return path.read_bytes(), path.name
    raise TypeError("ImageRun.data must resolve to bytes or a filesystem path")
