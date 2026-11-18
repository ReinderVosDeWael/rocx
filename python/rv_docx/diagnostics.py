"""Attach machine-readable context without wrapping user or operating-system errors."""


def annotate(error: Exception, rule: str, location: str) -> None:
    """Add missing diagnostic context while retaining identity and existing causes."""
    attributes = vars(error)
    attributes.setdefault("rule", rule)
    if attributes.get("location") is None:
        attributes["location"] = location
    attributes.setdefault("cause", str(error.__cause__) if error.__cause__ is not None else None)
