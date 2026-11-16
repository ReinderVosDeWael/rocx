These seven XML files were generated from CMI Docx revision
`3449b942034129ad74a28aeb253361035806e385` (0.6.16), using Python 3.12.14,
python-docx 1.2.0, and lxml. `generate_reference.py` verifies the source checkout
revision before regenerating them.

The comparison in `tests/reference.rs` checks paragraph content, section counts,
and section-break positions. It intentionally ignores XML prefixes, namespace
declarations, default template formatting, and ZIP metadata. Trailing excluded
sections and stateful predicates have explicitly documented Rust behavior; they
are tested in `tests/resolution.rs` rather than asserted equivalent to Python.
