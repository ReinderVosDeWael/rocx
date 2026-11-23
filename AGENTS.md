Before doing anything consult docs/PLAN.md . When anything you do or the engineer says changes the plan, change this file.

When you push a change that resolves a GitHub issue, close the issue after
the fix reaches the default branch. Link the fixing commit in the closure
comment if GitHub does not close the issue automatically.

Act as the repository maintainer. You may commit and push repository changes
and create and push release tags without asking for separate approval. Follow
the active release plan and runbook before pushing a release tag: the tag can
trigger automated publication. Keep unrelated work intact. This does not
authorize separate manual deployments or package publication.

Use `python-docx` to generate independent DOCX fixtures and to open and inspect
basic features of existing Word files and edited outputs. Use package-level
checks for relationships, opaque markup, and exact byte preservation;
python-docx inspection does not replace those checks or the rocx editor.

When adding functionality, update both `examples/all_features.rs` (and its support
modules) and `examples/all_features.py`, plus `examples/ALL_FEATURES.md` and the
parity/coverage checks. Keep visible examples clearly labeled. Run exact DOCX
byte-parity verification; normalized XML equality is not a substitute.
