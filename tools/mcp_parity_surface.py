#!/usr/bin/env python3
"""Inventory the compiled public editor API for MCP parity.

surface.json is versioned. Its `symbols` object maps Rust symbol to a shape
record with a `kind`. The three roots and their members use short keys (e.g.
`ImportedDocument::open`, `EditBatch::alignments`, `EditPreview::to_docx`);
other keys use qualified paths. Struct/variant fields and enum variants have
their own entries, with tuple positions written as numeric path components.
`public_paths` records public reexports; signatures refer to canonical
qualified names, not rustdoc IDs. Rustdoc is only a build-time input.

Starting with those roots, follow public fields, variants, aliases, inherent
members and generic signatures of local types. Foreign types and traits are
named but not expanded; trait implementations (including derived, auto and
blanket traits) are not editor methods. Do not walk unrelated crate exports.
"""

import argparse
from collections import defaultdict
import json
import os
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
SNAPSHOT = ROOT / "tests/mcp_parity/surface.json"
TMP = ROOT / "target/mcp-parity/tmp"
DOC_TARGET = ROOT / "target/mcp-parity/rustdoc"
TOOLCHAIN = "1.88.0"
RUSTDOC_FORMAT = 45
SCHEMA_VERSION = 1
ROOT_TYPES = ("ImportedDocument", "EditBatch", "EditPreview")


class Surface:
    def __init__(self, doc):
        if doc.get("format_version") != RUSTDOC_FORMAT:
            raise ValueError(f"expected rustdoc JSON format {RUSTDOC_FORMAT}, "
                             f"got {doc.get('format_version')}")
        if doc["includes_private"]:
            raise ValueError("expected rustdoc JSON without private items")
        self.index = {int(key): value for key, value in doc["index"].items()}
        self.paths = {int(key): value for key, value in doc["paths"].items()}
        self.local_crate = self.index[doc["root"]]["crate_id"]
        self.public_paths = defaultdict(set)
        self.modules(doc["root"], (self.index[doc["root"]]["name"],), frozenset())
        self.root_names = {}
        for root in ROOT_TYPES:
            name = ("rocx", "editor", root)
            matches = [item_id for item_id, paths in self.public_paths.items() if name in paths]
            if len(matches) != 1:
                raise ValueError(f"expected one public {'::'.join(name)}")
            self.root_names[matches[0]] = root
        self.symbols = {}
        self.visited = set()

    def members(self, item_id, seen):
        """Return visible name/ID pairs, resolving local glob reexports."""
        item = self.index.get(item_id)
        if item is None or item["crate_id"] != self.local_crate or item_id in seen:
            return {}
        kind, value = next(iter(item["inner"].items()))
        if kind == "enum":
            return {self.index[id_]["name"]: id_ for id_ in value["variants"]}
        if kind != "module":
            raise ValueError(f"unexpected glob target kind: {kind}")

        direct = {}
        globs = defaultdict(set)
        for child_id in value["items"]:
            child = self.index[child_id]
            if child["visibility"] != "public":
                continue
            use = child["inner"].get("use")
            if use is None:
                direct[child["name"]] = child_id
            elif use["id"] is not None:
                if use["is_glob"]:
                    for name, target in self.members(use["id"], seen | {item_id}).items():
                        globs[name].add(target)
                else:
                    direct[use["name"]] = use["id"]
        # Explicit names shadow glob imports. Ambiguous glob names aren't
        # accessible to a caller.
        for name, targets in globs.items():
            if len(targets) == 1:
                direct.setdefault(name, next(iter(targets)))
        return direct

    def modules(self, item_id, prefix, seen):
        for name, target in self.members(item_id, frozenset()).items():
            if not name:
                continue
            path = (*prefix, name)
            self.public_paths[target].add(path)
            child = self.index.get(target)
            if (child is not None and child["crate_id"] == self.local_crate
                    and "module" in child["inner"] and target not in seen | {item_id}):
                self.modules(target, path, seen | {item_id})

    def qualified(self, item_id):
        if item_id in self.root_names:
            return f"rocx::editor::{self.root_names[item_id]}"
        summary = self.paths.get(item_id)
        public = self.public_paths.get(item_id)
        if summary is not None:
            original = tuple(summary["path"])
            if summary["crate_id"] != self.local_crate or not public or original in public:
                return "::".join(original)
            # Prefer reexports preserving the type's actual name. Prefer the
            # editor spelling over exports from another namespace.
            return "::".join(min(public, key=lambda path: (
                path[-1] != original[-1],
                original[1:2] == ("editor",) and path[1:2] != ("editor",),
                len(path), path,
            )))
        if public:
            return "::".join(min(public, key=lambda path: (len(path), path)))
        raise ValueError("rustdoc reference has no qualified path")

    def normalize(self, value):
        """Preserve rustdoc signature structure, replacing path references."""
        if isinstance(value, list):
            return [self.normalize(child) for child in value]
        if not isinstance(value, dict):
            return value
        if "id" in value:
            if set(value) != {"path", "id", "args"}:
                raise ValueError(f"unexpected rustdoc reference keys: {sorted(value)}")
            target = value["id"]
            name = self.qualified(target)
            summary = self.paths.get(target)
            if target in self.index and self.index[target]["crate_id"] == self.local_crate:
                self.visit(target)
            elif summary is not None and summary["crate_id"] == self.local_crate:
                raise ValueError(f"missing local shape for {name}")
            args = self.normalize(value["args"])
            if args == {"angle_bracketed": {"args": [], "constraints": []}}:
                args = None
            return {"path": name, **({"args": args} if args is not None else {})}
        return {key: self.normalize(child) for key, child in value.items()}

    def add(self, symbol, kind, **record):
        if symbol in self.symbols:
            raise ValueError(f"duplicate Rust symbol {symbol}")
        self.symbols[symbol] = {"kind": kind, **record}

    def fields(self, symbol, shape):
        """Emit public field rows and return constructor shape metadata."""
        if shape in ("unit", "plain"):
            return {"shape": "unit"}
        kind, body = next(iter(shape.items()))
        if kind == "tuple":
            fields = enumerate(body)
            metadata = {"shape": "tuple", "arity": len(body)}
        elif kind in ("plain", "struct"):
            fields = ((self.index[id_]["name"], id_) for id_ in body["fields"])
            metadata = {"shape": "record"}
            if body["has_stripped_fields"]:
                metadata["has_private_fields"] = True
        else:
            raise ValueError(f"unexpected field shape: {kind}")
        for name, field_id in fields:
            if field_id is not None:
                field = self.index[field_id]
                if field["visibility"] in ("public", "default"):
                    self.add(f"{symbol}::{name}", "field",
                             type=self.normalize(field["inner"]["struct_field"]))
        return metadata

    def associated(self, symbol, items, *, receiver=None):
        for item_id in items:
            item = self.index[item_id]
            if item["visibility"] != "public" and not (
                receiver is None and item["visibility"] == "default"
            ):
                continue
            kind, value = next(iter(item["inner"].items()))
            name = f"{symbol}::{item['name']}"
            if kind == "function":
                signature = self.normalize({
                    key: value[key] for key in ("header", "sig", "generics")
                })
                if receiver is not None:
                    signature["impl"] = receiver
                if name not in self.symbols:
                    self.add(name, "method", signatures=[])
                self.symbols[name]["signatures"].append(signature)
            elif kind in ("assoc_const", "assoc_type"):
                self.add(name, kind, declaration=self.normalize(value))
            else:
                raise ValueError(f"unexpected associated item kind: {kind}")

    def visit(self, item_id):
        if item_id in self.visited:
            return
        self.visited.add(item_id)
        item = self.index[item_id]
        qualified = self.qualified(item_id)
        symbol = self.root_names.get(item_id, qualified)
        kind, value = next(iter(item["inner"].items()))
        record = {"public_paths": sorted("::".join(path) for path in self.public_paths[item_id])}
        if "generics" in value:
            record["generics"] = self.normalize(value["generics"])
        if "#[non_exhaustive]" in item["attrs"]:
            record["non_exhaustive"] = True
        if kind in ("struct", "union"):
            shape = value["kind"] if kind == "struct" else {"struct": value}
            record.update(self.fields(symbol, shape))
        elif kind == "enum":
            for variant_id in value["variants"]:
                variant = self.index[variant_id]
                variant_name = f"{symbol}::{variant['name']}"
                variant_value = variant["inner"]["variant"]
                shape = self.fields(variant_name, variant_value["kind"])
                aliases = sorted("::".join(path) for path in self.public_paths[variant_id])
                self.add(variant_name, "variant", **shape,
                         discriminant=variant_value["discriminant"],
                         **({"public_paths": aliases} if aliases else {}),
                         **({"non_exhaustive": True} if "#[non_exhaustive]" in variant["attrs"] else {}))
        elif kind == "type_alias":
            record["type"] = self.normalize(value["type"])
        elif kind == "trait":
            record.update(self.normalize({key: value[key] for key in (
                "bounds", "is_auto", "is_unsafe", "is_dyn_compatible",
            )}))
            self.associated(symbol, value["items"])
        else:
            raise ValueError(f"unsupported reachable Rust kind {kind}: {qualified}")
        self.add(symbol, kind, **record)
        for impl_id in value.get("impls", []):
            impl = self.index[impl_id]["inner"]["impl"]
            if impl["trait"] is None and impl["items"]:
                receiver = self.normalize({key: impl[key] for key in ("for", "generics")})
                self.associated(symbol, impl["items"], receiver=receiver)

    def extract(self):
        for item_id in self.root_names:
            self.visit(item_id)
        for record in self.symbols.values():
            if record["kind"] == "method":
                record["signatures"].sort(key=lambda value: json.dumps(value, sort_keys=True))
        return {"schema_version": SCHEMA_VERSION, "symbols": self.symbols}


def build_rustdoc():
    TMP.mkdir(parents=True, exist_ok=True)
    subprocess.run([
        "cargo", f"+{TOOLCHAIN}", "rustdoc", "--locked", "-p", "rocx", "--lib",
        "--target-dir", str(DOC_TARGET), "--", "-Z", "unstable-options",
        "--output-format", "json", "--document-hidden-items",
    ], cwd=ROOT, env={**os.environ, "TMPDIR": str(TMP), "RUSTC_BOOTSTRAP": "rocx"}, check=True)
    return DOC_TARGET / "doc/rocx.json"


def encode(surface):
    # One row per symbol keeps diffs focused while preserving structured
    # signatures for consumers.
    rows = [f"    {json.dumps(name)}: {json.dumps(record, sort_keys=True)}"
            for name, record in sorted(surface["symbols"].items())]
    return ('{\n  "schema_version": ' + str(surface["schema_version"])
            + ',\n  "symbols": {\n' + ",\n".join(rows) + "\n  }\n}\n")


def check(snapshot, surface):
    saved = snapshot.read_text(encoding="utf-8")
    if saved == encode(surface):
        print(f"Editor surface matches {len(surface['symbols'])} symbols")
        return 0
    previous = json.loads(saved)
    before, after = previous["symbols"], surface["symbols"]
    for names, label in (
        (after.keys() - before.keys(), "added"),
        (before.keys() - after.keys(), "removed"),
        ({key for key in before.keys() & after.keys() if before[key] != after[key]}, "changed"),
    ):
        for name in sorted(names):
            print(f"  {label}: {name}", file=sys.stderr)
    print("Editor surface drift; review parity mappings and regenerate with --write.",
          file=sys.stderr)
    return 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="replace the generated snapshot")
    mode.add_argument("--check", action="store_true", help="fail on compiled surface drift")
    parser.add_argument("--rustdoc-json", type=Path, help="use existing pinned-format JSON")
    parser.add_argument("--snapshot", type=Path, default=SNAPSHOT, help="override snapshot path")
    args = parser.parse_args()
    try:
        source = args.rustdoc_json if args.rustdoc_json else build_rustdoc()
        surface = Surface(json.loads(source.read_text(encoding="utf-8"))).extract()
        if args.check:
            return check(args.snapshot, surface)
        args.snapshot.write_text(encode(surface), encoding="utf-8")
        print(f"Wrote {len(surface['symbols'])} editor symbols to {args.snapshot}")
        return 0
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"Editor surface extraction failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
