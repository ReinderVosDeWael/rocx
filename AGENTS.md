Before doing anything consult docs/PLAN.md . When anything you do or the engineer says changes the plan, change this file.

When adding functionality, update both `examples/all_features.rs` (and its support
modules) and `examples/all_features.py`, plus `examples/ALL_FEATURES.md` and the
parity/coverage checks. Keep visible examples clearly labeled. Run exact DOCX
byte-parity verification; normalized XML equality is not a substitute.
