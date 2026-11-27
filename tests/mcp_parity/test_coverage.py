"""Check reviewed MCP parity claims without running installed fixtures."""

from contextlib import redirect_stderr, redirect_stdout
from importlib.util import module_from_spec, spec_from_file_location
from io import StringIO
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch


INVENTORY = Path(__file__).resolve().parent
SPEC = spec_from_file_location(
    "check_mcp_parity", INVENTORY.parents[1] / "tools/check_mcp_parity.py",
)
CHECK = module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)
InvalidLedger, unique_object, validate = CHECK.InvalidLedger, CHECK.unique_object, CHECK.validate


class CoverageLedgerTests(unittest.TestCase):
    def setUp(self):
        self.surface = json.loads((INVENTORY / "surface.json").read_text(encoding="utf-8"))
        self.ledger = json.loads((INVENTORY / "coverage.json").read_text(encoding="utf-8"))

    def test_checked_in_inventory_is_consistent(self):
        validate(self.surface, self.ledger)

    def test_closure_requires_no_unfinished_or_unverified_rows(self):
        ledger = {"rows": {}, "unverified_symbols": []}
        CHECK.require_complete(ledger)
        ledger["rows"]["pending"] = {"status": "planned"}
        ledger["rows"]["under_test"] = {"status": "partial"}
        ledger["unverified_symbols"].append("unreviewed")
        with self.assertRaisesRegex(
            InvalidLedger, "1 partial rows, 1 planned rows, 1 unverified Rust symbols",
        ):
            CHECK.require_complete(ledger)
        ledger["rows"]["pending"]["status"] = "covered"
        ledger["rows"]["under_test"]["status"] = "covered"
        with self.assertRaisesRegex(InvalidLedger, "1 unverified Rust symbols"):
            CHECK.require_complete(ledger)
        ledger["unverified_symbols"].clear()
        CHECK.require_complete(ledger)

    def test_exclusion_requires_a_tracked_scope_decision(self):
        row = self.ledger["rows"]["rocx::editor::TextInsertion::position"]
        row.update(status="excluded", gap="Example scope decision.", positive=[], negative=[])
        with self.assertRaisesRegex(InvalidLedger, "excluded row needs a decision linked"):
            validate(self.surface, self.ledger)
        row["decision"] = "https://github.com/ReinderVosDeWael/rocx/issues/72#issuecomment-1"
        validate(self.surface, self.ledger)
        self.ledger["rows"]["EditBatch::ranges"]["decision"] = row["decision"]
        with self.assertRaisesRegex(InvalidLedger, "only excluded rows can record a scope decision"):
            validate(self.surface, self.ledger)

    def test_added_method_or_slice_requires_a_curated_mapping(self):
        for symbol, kind in [
            ("ImportedDocument::unmapped_method", "method"),
            ("EditBatch::unmapped_slice", "field"),
            ("EditPreview::unmapped_result", "method"),
        ]:
            with self.subTest(symbol=symbol):
                self.surface["symbols"][symbol] = {"kind": kind}
                # Recording a batch field or method as generically unverified
                # is not a substitute for reviewing its operation mapping.
                self.ledger["unverified_symbols"].append(symbol)
                with self.assertRaisesRegex(InvalidLedger, f"missing curated mapping: {symbol}"):
                    validate(self.surface, self.ledger)
                del self.surface["symbols"][symbol]
                self.ledger["unverified_symbols"].remove(symbol)

    def test_new_reachable_members_are_not_covered_by_their_parent(self):
        for symbol, kind in [
            ("rocx::editor::TextRangeEdit::new_field", "field"),
            ("rocx::editor::TableOperation::NewVariant", "variant"),
        ]:
            with self.subTest(symbol=symbol):
                self.surface["symbols"][symbol] = {"kind": kind}
                with self.assertRaisesRegex(
                    InvalidLedger, f"missing explicit coverage obligation: {symbol}",
                ):
                    validate(self.surface, self.ledger)
                self.ledger["unverified_symbols"].append(symbol)
                validate(self.surface, self.ledger)

    def test_removed_rust_member_leaves_a_stale_row(self):
        symbol = "EditBatch::run_text"
        del self.surface["symbols"][symbol]
        with self.assertRaisesRegex(InvalidLedger, f"unknown Rust symbol: {symbol}"):
            validate(self.surface, self.ledger)

    def test_row_cannot_be_both_mapped_and_unverified(self):
        symbol = "rocx::editor::TextInsertion::position"
        self.ledger["unverified_symbols"].append(symbol)
        with self.assertRaisesRegex(InvalidLedger, f"duplicate disposition .*: {symbol}"):
            validate(self.surface, self.ledger)

    def test_missing_evidence_or_fixture_id_fails(self):
        row = self.ledger["rows"]["EditBatch::ranges"]
        row["positive"] = []
        row["negative"] = ["typo-in-fixture-id"]
        with self.assertRaisesRegex(
            InvalidLedger,
            r"(?s)EditBatch::ranges: covered row needs positive fixtures.*unknown negative fixture",
        ):
            validate(self.surface, self.ledger)

    def test_fixture_id_must_reference_an_existing_installed_test(self):
        for reference, reason in [
            ("tests/mcp/test_stdio.py::test_nonexistent", "test not found"),
            ("tests/mcp/test_nonexistent.py::test_discovery", "test not found"),
            ("tests/mcp/../mcp_parity/test_coverage.py::test_discovery", "not an installed MCP"),
        ]:
            with self.subTest(reference=reference):
                self.ledger["fixtures"]["discovery"] = reference
                with self.assertRaisesRegex(InvalidLedger, f"fixture discovery: {reason}"):
                    validate(self.surface, self.ledger)

    def test_alias_cannot_claim_more_than_its_required_batch_slices(self):
        row = self.ledger["rows"]["ImportedDocument::preview_with_paragraphs"]
        row["status"] = "covered"
        row.pop("gap", None)
        self.ledger["rows"]["EditBatch::paragraphs"].update(status="partial", gap="Incomplete.")
        with self.assertRaisesRegex(
            InvalidLedger, "covered row depends on non-covered slice paragraphs",
        ):
            validate(self.surface, self.ledger)

    def test_main_batch_cannot_claim_coverage_with_planned_slices(self):
        row = self.ledger["rows"]["ImportedDocument::preview_edits"]
        row["status"] = "covered"
        row.pop("gap", None)
        self.ledger["rows"]["EditBatch::fields"].update(status="planned", gap="Not implemented.")
        with self.assertRaisesRegex(
            InvalidLedger, "preview_edits: covered row depends on non-covered slice fields",
        ):
            validate(self.surface, self.ledger)

    def test_alias_must_identify_a_real_operation_and_slices(self):
        row = self.ledger["rows"]["ImportedDocument::preview"]
        row["alias_of"] = "ImportedDocument::not_a_method"
        row["batch_slices"] = ["not_a_slice"]
        with self.assertRaisesRegex(InvalidLedger, r"(?s)unknown alias target.*unknown batch slice"):
            validate(self.surface, self.ledger)

    def test_cyclic_alias_is_invalid(self):
        self.ledger["rows"]["ImportedDocument::preview_edits"]["alias_of"] = "ImportedDocument::preview"
        with self.assertRaisesRegex(InvalidLedger, "cyclic alias"):
            validate(self.surface, self.ledger)

    def test_duplicate_json_keys_cannot_hide_clashing_rows(self):
        with self.assertRaisesRegex(InvalidLedger, "duplicate JSON key: EditBatch::ranges"):
            json.loads('{"EditBatch::ranges": {}, "EditBatch::ranges": {}}', object_pairs_hook=unique_object)


class CoverageGroupTests(unittest.TestCase):
    def setUp(self):
        self.members = ["rocx::editor::Payload::first", "rocx::editor::Payload::second"]
        self.template = {
            "scope": "body", "target": "payload field", "mcp": "preview_edits",
            "status": "covered", "positive": ["positive"], "negative": ["negative"],
        }
        self.group = {"symbols": self.members.copy(), "template": self.template}
        rows = {
            "ImportedDocument::open": self.template.copy(),
            "EditBatch::ranges": self.template.copy(),
            "EditPreview::report": self.template.copy(),
        }
        self.ledger = {
            "version": 1, "rows": rows, "unverified_symbols": [], "groups": [self.group],
            "fixtures": {
                "positive": "tests/mcp/test_stdio.py::test_mixed_native_bytes_and_preservation",
                "negative": "tests/mcp/test_stdio.py::test_scalar_edits_native_bytes_and_retry",
            },
        }
        self.surface = {"schema_version": 1, "symbols": dict.fromkeys([*rows, *self.members], {})}

    def run_checker(self, *args):
        stdout, stderr = StringIO(), StringIO()
        with (
            patch.object(sys, "argv", ["check_mcp_parity.py", *args]),
            patch.object(CHECK, "read_json", side_effect=[self.surface, self.ledger]),
            redirect_stdout(stdout),
            redirect_stderr(stderr),
        ):
            result = CHECK.main()
        return result, stdout.getvalue(), stderr.getvalue()

    def test_groups_count_as_individual_rows_in_progress_and_closure(self):
        self.template.update(status="partial", gap="Needs more evidence.")
        result, stdout, stderr = self.run_checker()
        self.assertEqual((result, stderr), (0, ""))
        self.assertIn("3 covered / 2 partial / 0 planned / 0 excluded", stdout)
        result, stdout, stderr = self.run_checker("--require-complete")
        self.assertEqual((result, stdout), (1, ""))
        self.assertIn("MCP parity closure blocked: 2 partial rows", stderr)
        self.template["status"] = "covered"
        del self.template["gap"]
        result, stdout, stderr = self.run_checker("--require-complete")
        self.assertEqual((result, stderr), (0, ""))
        self.assertIn("5 covered / 0 partial / 0 planned / 0 excluded", stdout)

    def test_group_dispositions_cannot_overlap(self):
        key = self.members[0]
        cases = [
            (self.group["symbols"], key, "duplicate entries"),
            (self.ledger["groups"], {"symbols": [key], "template": self.template},
             f"duplicate disposition .*: {key}"),
            (self.ledger["unverified_symbols"], key, f"duplicate disposition .*: {key}"),
        ]
        for entries, duplicate, message in cases:
            with self.subTest(duplicate=duplicate):
                entries.append(duplicate)
                with self.assertRaisesRegex(InvalidLedger, message):
                    validate(self.surface, self.ledger)
                entries.pop()
        self.ledger["rows"][key] = self.template
        with self.assertRaisesRegex(InvalidLedger, f"duplicate disposition .*: {key}"):
            validate(self.surface, self.ledger)

    def test_groups_require_nonempty_exact_symbols(self):
        for symbols, message in [
            ([], "expected at least one explicit Rust symbol"),
            ("rocx::editor::Payload", "expected a list of nonempty strings"),
            (["rocx::editor::Payload::*"], "unknown Rust symbol"),
            (["rocx::editor::Payload::obsolete"], "unknown Rust symbol"),
        ]:
            with self.subTest(symbols=symbols):
                self.group["symbols"] = symbols
                with self.assertRaisesRegex(InvalidLedger, message):
                    validate(self.surface, self.ledger)

    def test_group_templates_require_valid_row_fields_and_evidence(self):
        for overrides, message in [
            ({"symbols": self.members}, "missing or unknown row fields"),
            ({"status": "unknown"}, "unknown status"),
            ({"positive": [], "negative": ["missing"]},
             r"(?s)covered row needs positive fixtures.*unknown negative fixture missing"),
        ]:
            with self.subTest(overrides=overrides):
                self.group["template"] = {**self.template, **overrides}
                with self.assertRaisesRegex(InvalidLedger, message):
                    validate(self.surface, self.ledger)
        self.group["template"] = {"status": "covered"}
        with self.assertRaisesRegex(InvalidLedger, "missing or unknown row fields"):
            validate(self.surface, self.ledger)

    def test_group_cannot_replace_a_curated_row(self):
        for key in list(self.ledger["rows"]):
            with self.subTest(key=key):
                self.group["symbols"].append(key)
                row = self.ledger["rows"].pop(key)
                with self.assertRaisesRegex(
                    InvalidLedger, f"curated mapping must be an individual row: {key}",
                ):
                    validate(self.surface, self.ledger)
                self.ledger["rows"][key] = row
                self.group["symbols"].pop()

    def test_a_new_sibling_blocks_closure_until_explicitly_listed(self):
        key = "rocx::editor::Payload::third"
        self.surface["symbols"][key] = {"kind": "field"}
        result, stdout, stderr = self.run_checker("--require-complete")
        self.assertEqual((result, stdout), (1, ""))
        self.assertIn(f"missing explicit coverage obligation: {key}", stderr)
        self.group["symbols"].append(key)
        result, stdout, stderr = self.run_checker("--require-complete")
        self.assertEqual((result, stderr), (0, ""))
        self.assertIn("6 covered", stdout)


if __name__ == "__main__":
    unittest.main()
