"""Python interface to the native Rust DOCX engine."""
from . import declarative, editor
from .options import ReadLimits, RenderOptions

__version__ = "0.4.0"
__backend__ = "rust"
__all__ = ["declarative", "editor", "ReadLimits", "RenderOptions"]
