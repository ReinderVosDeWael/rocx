"""Keep upstream provenance intact while retiring an intentional CMI-only quirk."""
from pathlib import Path
import pytest


def pytest_collection_modifyitems(items):
    for item in items:
        if item.path.parent == Path(__file__).parent and item.path.name == "test_condition.py" and item.name == "test_section_condition_false":
            item.add_marker(pytest.mark.skip(reason="CMI trailing empty section retired; native section exclusion is covered in test_resolution.py"))
