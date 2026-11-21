"""Python interface to the Rust DOCX engine (not the upstream CMI renderer)."""
from . import declarative
from .options import ReadLimits, RenderOptions

__version__ = "0.2.0"
__backend__ = "rust"
__all__ = ["declarative", "ReadLimits", "RenderOptions"]
