"""Check reviewed MCP parity claims without running installed fixtures."""

from importlib.util import module_from_spec, spec_from_file_location
import json
from pathlib import Path
import unittest


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


if __name__ == "__main__":
    unittest.main()
