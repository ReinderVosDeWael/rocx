"""Explicit rendering policy; these immutable options do not contain producers."""
import dataclasses


@dataclasses.dataclass(frozen=True)
class ReadLimits:
    """Application package-read budgets, not OOXML schema limits."""

    max_parts: int = 10_000
    max_part_bytes: int = 64 * 1024 * 1024
    max_expanded_bytes: int = 256 * 1024 * 1024


@dataclasses.dataclass(frozen=True)
class RenderOptions:
    """Rendering profile, optional timestamp override and template-read budgets.

    Only the Transitional profile is implemented. Strict is explicitly rejected.
    Omitting timestamp preserves the timestamp already captured in the snapshot.
    """

    profile: str = "transitional"
    timestamp: str | None = None
    read_limits: ReadLimits = dataclasses.field(default_factory=ReadLimits)
