"""Python interface to the native Rust DOCX engine."""
from . import declarative
from .options import ReadLimits, RenderOptions

__version__ = "0.3.0"
__backend__ = "rust"
__all__ = ["declarative", "ReadLimits", "RenderOptions"]
