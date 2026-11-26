"""Exercise parity drift against real compiled Rust signatures."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
EXTRACTOR = ROOT / "tools/mcp_parity_surface.py"
TMP = ROOT / "target/mcp-parity/tmp"
TOOLCHAIN = "1.88.0"

SOURCE = """
mod payload {
    pub enum Choice<T> {
        Empty,
        Data { child: Option<Box<crate::editor::Payload<T>>> },
        Tuple(T),
    }
    pub struct Record<T> {
        pub choice: Choice<T>,
        private: bool,
    }
    pub struct Preview;
}
pub mod editor {
    pub use crate::payload::*;
    pub use crate::payload::Preview as EditPreview;
    pub use crate::payload::Record as Payload;
    pub type Result<T> = std::result::Result<T, &'static str>;

    pub struct ImportedDocument;
    pub struct EditBatch<'a> { pub records: &'a [Payload<u16>] }

    macro_rules! preview {
        ($name:ident) => {
            pub fn $name<'a, T>(&self, batch: EditBatch<'a>, _: T) -> Result<EditPreview>
            where T: Into<Payload<u16>>
            { let _ = batch; Ok(EditPreview) }
        }
    }
    impl ImportedDocument {
        preview!(preview);
        #[doc(hidden)]
        pub fn hidden_api(&self) {}
        pub(crate) fn private_api(&self) {}
    }
    impl EditPreview {
        pub fn data(&self) -> Option<Choice<u16>> { None }
    }
}
"""


class CompiledSurfaceTests(unittest.TestCase):
    def setUp(self):
        TMP.mkdir(parents=True, exist_ok=True)
        temporary = tempfile.TemporaryDirectory(prefix="surface-test-", dir=TMP)
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.snapshot = self.directory / "surface.json"

    def compile(self, source):
        subprocess.run([
            "rustup", "run", TOOLCHAIN, "rustdoc", "--crate-name", "rocx",
            "--edition", "2024", "-Z", "unstable-options", "--output-format", "json",
            "--document-hidden-items", "-o", str(self.directory), "-",
        ], input=source, text=True, check=True, cwd=ROOT,
            env={**os.environ, "TMPDIR": str(TMP), "RUSTC_BOOTSTRAP": "rocx"})
        return self.directory / "rocx.json"

    def run_extractor(self, mode, doc):
        return subprocess.run([
            sys.executable, str(EXTRACTOR), mode, "--rustdoc-json", str(doc),
            "--snapshot", str(self.snapshot),
        ], cwd=ROOT, text=True, capture_output=True)

    def write_baseline(self):
        result = self.run_extractor("--write", self.compile(SOURCE))
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_compiled_macros_reexports_generics_and_recursive_shapes(self):
        self.write_baseline()
        snapshot = json.loads(self.snapshot.read_text(encoding="utf-8"))
        symbols = snapshot["symbols"]
        self.assertEqual(symbols["EditBatch::records"]["kind"], "field")
        self.assertEqual(symbols["EditPreview::data"]["kind"], "method")
        self.assertIn("ImportedDocument::hidden_api", symbols)
        self.assertNotIn("ImportedDocument::private_api", symbols)
        self.assertNotIn("rocx::editor::Record::private", symbols)
        self.assertEqual(symbols["rocx::editor::Record"]["public_paths"],
                         ["rocx::editor::Payload", "rocx::editor::Record"])
        self.assertEqual(symbols["rocx::editor::Choice::Tuple::0"]["type"], {"generic": "T"})
        self.assertIn("rocx::editor::Choice::Data::child", symbols)
        self.assertFalse(any(key.startswith(("alloc::", "core::", "std::")) for key in symbols))
        signature, = symbols["ImportedDocument::preview"]["signatures"]
        signature_text = json.dumps(signature)
        self.assertIn("'a", signature_text)
        self.assertIn("rocx::editor::Record", signature_text)
        self.assertIn("rocx::editor::Result", signature_text)
        self.assertIn("core::convert::Into", signature_text)
        self.assertNotIn('"id":', json.dumps(symbols))

        original_ids = json.loads((self.directory / "rocx.json").read_text())["paths"]
        # An unrelated reachable-in-the-crate export can change rustdoc IDs but
        # must not change any editor row.
        different_doc = self.compile("pub struct Unrelated { pub field: u8 }\n" + SOURCE)
        self.assertNotEqual(original_ids, json.loads(different_doc.read_text())["paths"])
        checked = self.run_extractor("--check", different_doc)
        self.assertEqual(checked.returncode, 0, checked.stderr)

    def test_compiled_field_and_variant_additions_each_fail_drift_check(self):
        self.write_baseline()
        baseline = self.snapshot.read_bytes()
        cases = (
            ("private: bool,", "private: bool,\n        pub extra: Option<u16>,",
             "rocx::editor::Record::extra"),
            ("        Empty,", "        Empty,\n        Added { value: u16 },",
             "rocx::editor::Choice::Added"),
        )
        for original, replacement, symbol in cases:
            with self.subTest(symbol=symbol):
                doc = self.compile(SOURCE.replace(original, replacement))
                checked = self.run_extractor("--check", doc)
                self.assertEqual(checked.returncode, 1, checked.stderr)
                self.assertIn(f"added: {symbol}\n", checked.stderr)
                self.assertEqual(self.snapshot.read_bytes(), baseline)


if __name__ == "__main__":
    unittest.main()
