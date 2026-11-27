#!/usr/bin/env python3
"""Check reviewed MCP coverage against the generated Rust editor surface.

This checks the inventory and fixture references, not whether fixtures pass or
their assertions prove a claim. Run the extractor's drift check and installed
MCP suite separately.
"""

import argparse
import ast
import json
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / "tests/mcp_parity"
CURATED_OWNERS = {"ImportedDocument", "EditBatch", "EditPreview"}
LEDGER_FIELDS = {"version", "rows", "fixtures", "unverified_symbols"}
STATUS = {"planned", "partial", "covered", "excluded"}
REQUIRED = {"scope", "target", "mcp", "status", "positive", "negative"}
OPTIONAL = {"gap", "alias_of", "batch_slices", "decision"}
ISSUE_DECISION = re.compile(
    r"https://github\.com/ReinderVosDeWael/rocx/issues/72#issuecomment-\d+\Z"
)


class InvalidLedger(ValueError):
    """Raised when a manifest cannot be validated."""


def unique_object(pairs):
    obj = {}
    for key, value in pairs:
        if key in obj:
            raise InvalidLedger(f"duplicate JSON key: {key}")
        obj[key] = value
    return obj


def read_json(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    except (OSError, ValueError) as exc:
        raise InvalidLedger(f"{path}: {exc}") from exc


def nonempty_string(value):
    return isinstance(value, str) and bool(value.strip())


def string_set(value, label):
    if not isinstance(value, list) or not all(nonempty_string(item) for item in value):
        raise InvalidLedger(f"{label}: expected a list of nonempty strings")
    result = set(value)
    if len(result) != len(value):
        raise InvalidLedger(f"{label}: duplicate entries")
    return result


def fixture_errors(fixtures, root):
    """Resolve pytest node IDs without importing the installed-client suite."""
    test_root = (root / "tests/mcp").resolve()
    modules = {}
    for fixture, reference in fixtures.items():
        if not isinstance(reference, str):
            yield f"fixture {fixture}: expected a tests/mcp/test_*.py::test_* reference"
            continue
        path, sep, fn = reference.partition("::")
        if not sep or not re.fullmatch(r"test_\w+", fn):
            yield f"fixture {fixture}: invalid test reference {reference}"
            continue
        try:
            relative = Path(path)
            if relative.parts[:2] != ("tests", "mcp") or ".." in relative.parts:
                raise ValueError(path)
            file = (root / relative).resolve()
        except (OSError, RuntimeError, ValueError):
            yield f"fixture {fixture}: not an installed MCP test file: {path}"
            continue
        if not file.is_relative_to(test_root) or not file.match("test_*.py"):
            yield f"fixture {fixture}: not an installed MCP test file: {path}"
            continue
        if file not in modules:
            try:
                tree = ast.parse(file.read_text(encoding="utf-8"), filename=path)
                modules[file] = {
                    node.name for node in tree.body
                    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
                }
            except (OSError, SyntaxError, UnicodeError):
                modules[file] = set()
        if fn not in modules[file]:
            yield f"fixture {fixture}: test not found: {reference}"


def effective_rows(ledger):
    """Expand each {symbols: [...], template: {...}} entry without overrides."""
    rows = ledger["rows"].copy()
    groups = ledger.get("groups", [])
    if not isinstance(groups, list):
        raise InvalidLedger("groups: expected a list of coverage groups")
    for i, group in enumerate(groups):
        label = f"groups[{i}]"
        if not isinstance(group, dict) or group.keys() != {"symbols", "template"}:
            raise InvalidLedger(f"{label}: expected symbols and template fields")
        members = string_set(group["symbols"], f"{label} symbols")
        if not members:
            raise InvalidLedger(f"{label}: expected at least one explicit Rust symbol")
        for key in sorted(members):
            if key in rows:
                previous = "row" if key in ledger["rows"] else "group"
                raise InvalidLedger(f"{label}: duplicate disposition ({previous} and group): {key}")
            rows[key] = group["template"]
    return rows


def validate(surface, ledger, root=ROOT):
    """Return effective rows, or raise InvalidLedger if inventories disagree."""
    if (
        not isinstance(surface, dict)
        or type(surface.get("schema_version")) is not int
        or surface["schema_version"] != 1
        or not isinstance(surface.get("symbols"), dict)
    ):
        raise InvalidLedger("surface: expected a version 1 symbols object")
    if (
        not isinstance(ledger, dict)
        or not LEDGER_FIELDS <= ledger.keys() <= LEDGER_FIELDS | {"groups"}
        or type(ledger["version"]) is not int
        or ledger["version"] != 1
    ):
        raise InvalidLedger("coverage: expected version 1 ledger")
    individual_rows, fixtures = ledger["rows"], ledger["fixtures"]
    if not isinstance(individual_rows, dict) or not isinstance(fixtures, dict):
        raise InvalidLedger("coverage: expected rows and fixtures objects")
    unverified = string_set(ledger["unverified_symbols"], "unverified_symbols")
    rows = effective_rows(ledger)
    symbols = set(surface["symbols"])
    curated = {
        key for key in symbols
        if "::" in key and key.partition("::")[0] in CURATED_OWNERS
    }
    errors = []
    for owner in sorted(CURATED_OWNERS - {key.partition("::")[0] for key in curated}):
        errors.append(f"surface: no {owner} member symbols")
    for key in sorted(curated - individual_rows.keys()):
        if key in rows:
            errors.append(f"curated mapping must be an individual row: {key}")
        else:
            errors.append(f"missing curated mapping: {key}")
    for key in sorted(symbols - curated - rows.keys() - unverified):
        errors.append(f"missing explicit coverage obligation: {key}")
    for key in sorted((rows.keys() | unverified) - symbols):
        errors.append(f"unknown Rust symbol: {key}")
    for key in sorted(rows.keys() & unverified):
        disposition = "row" if key in individual_rows else "group"
        errors.append(f"duplicate disposition ({disposition} and unverified): {key}")

    errors.extend(fixture_errors(fixtures, root))
    checked_rows = {}
    for key, row in rows.items():
        if not isinstance(row, dict) or not REQUIRED <= row.keys() <= REQUIRED | OPTIONAL:
            errors.append(f"{key}: missing or unknown row fields")
            continue
        if any(not nonempty_string(row[field]) for field in ("scope", "target", "mcp", "status")):
            errors.append(f"{key}: scope, target, mcp and status must be nonempty strings")
            continue
        status = row["status"]
        if status not in STATUS:
            errors.append(f"{key}: unknown status {status}")
            continue
        if status != "covered" and not nonempty_string(row.get("gap")):
            errors.append(f"{key}: {status} row must explain its gap")
        if status == "covered" and "gap" in row:
            errors.append(f"{key}: covered row must not declare a gap")
        if status == "excluded":
            if not (isinstance(row.get("decision"), str) and ISSUE_DECISION.fullmatch(row["decision"])):
                errors.append(f"{key}: excluded row needs a decision linked to a comment on #72")
        elif "decision" in row:
            errors.append(f"{key}: only excluded rows can record a scope decision")
        for direction in ("positive", "negative"):
            try:
                refs = string_set(row[direction], f"{key} {direction}")
            except InvalidLedger as exc:
                errors.append(str(exc))
                continue
            if not refs and status in {"partial", "covered"}:
                errors.append(f"{key}: {status} row needs {direction} fixtures")
            for ref in sorted(refs - fixtures.keys()):
                errors.append(f"{key}: unknown {direction} fixture {ref}")
        checked_rows[key] = row

    for key, row in checked_rows.items():
        alias = row.get("alias_of")
        if "alias_of" in row:
            if not nonempty_string(alias) or alias not in checked_rows:
                errors.append(f"{key}: unknown alias target {alias}")
            else:
                seen = {key}
                while alias:
                    if alias in seen:
                        errors.append(f"{key}: cyclic alias")
                        break
                    seen.add(alias)
                    alias = checked_rows[alias].get("alias_of")
                    if not nonempty_string(alias) or alias not in checked_rows:
                        break
        try:
            slices = string_set(row.get("batch_slices", []), f"{key} batch_slices")
        except InvalidLedger as exc:
            errors.append(str(exc))
            continue
        if key == "ImportedDocument::preview_edits":
            slices = {
                symbol.split("::", 1)[1] for symbol in curated if symbol.startswith("EditBatch::")
            }
        for field in sorted(slices):
            target = f"EditBatch::{field}"
            if target not in checked_rows or target not in symbols:
                errors.append(f"{key}: unknown batch slice {field}")
            elif row["status"] == "covered" and checked_rows[target]["status"] != "covered":
                errors.append(f"{key}: covered row depends on non-covered slice {field}")

    if errors:
        raise InvalidLedger("\n".join(errors))
    return rows


def require_complete(ledger):
    """Fail the closure gate for an internally consistent but unfinished ledger."""
    rows = effective_rows(ledger)
    remaining = [
        f"{count} {status} rows"
        for status in ("partial", "planned")
        if (count := sum(row["status"] == status for row in rows.values()))
    ]
    if ledger["unverified_symbols"]:
        remaining.append(f"{len(ledger['unverified_symbols'])} unverified Rust symbols")
    if remaining:
        raise InvalidLedger("MCP parity closure blocked: " + ", ".join(remaining))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--surface", type=Path, default=INVENTORY / "surface.json")
    parser.add_argument("--coverage", type=Path, default=INVENTORY / "coverage.json")
    parser.add_argument("--require-complete", action="store_true",
                        help="also reject outstanding coverage obligations")
    args = parser.parse_args()
    try:
        surface = read_json(args.surface)
        ledger = read_json(args.coverage)
        rows = validate(surface, ledger)
        if args.require_complete:
            require_complete(ledger)
    except InvalidLedger as exc:
        print(exc, file=sys.stderr)
        return 1
    counts = " / ".join(
        f"{sum(row['status'] == status for row in rows.values())} {status}"
        for status in ("covered", "partial", "planned", "excluded")
    )
    print(f"MCP parity ledger valid: {counts}; "
          f"{len(ledger['unverified_symbols'])} unverified Rust symbols")
    return 0


if __name__ == "__main__":
    sys.exit(main())
