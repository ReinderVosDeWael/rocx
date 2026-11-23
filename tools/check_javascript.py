"""Compare all JavaScript example outputs to native Rust and independently read them."""
import argparse
from pathlib import Path

from check_examples import exact_bytes, verify_javascript_workflow
from validate_ooxml import Validator, default_directory


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_dir", type=Path)
    args = parser.parse_args()
    reference = (args.output_dir / "rust.docx").read_bytes()
    validator = Validator(default_directory())
    for name in ("rust", "python", "node", "browser-dev", "browser"):
        path = args.output_dir / f"{name}.docx"
        data = path.read_bytes()
        digest = exact_bytes(reference, data)
        verify_javascript_workflow(data, standalone=True)
        checked = validator.validate(path)
        print(f"{name}: {len(data)} bytes; SHA-256 {digest}; {len(checked)} schema-covered parts and OPC relationships")


if __name__ == "__main__":
    main()
