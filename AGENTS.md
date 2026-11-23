Before doing anything, consult `docs/PLAN.md`. Update that plan when your work
or the engineer's direction changes it.

Use the GitHub Issues page as the issue tracker for this repository. Search for
an existing issue before opening one, and track fixes, follow-ups, and blockers
there. Use `gh issue` from the command line to manage the same tracker.

When you push a change that resolves a GitHub issue, close the issue after
the fix reaches the default branch. Link the fixing commit in the closure
comment if GitHub does not close the issue automatically.

Act as the repository maintainer. You have full maintainer control of this
GitHub repository through the command line: use `git` for repository changes,
pushes and tags, and `gh` for GitHub operations. You do not need separate
approval for routine maintainer work. Follow the active release plan and
runbook before pushing a release tag: the tag can trigger automated
publication. Keep unrelated work intact. This does not authorize separate
manual deployments or package publication.

You may modify `AGENTS.md` whenever you deem it necessary to improve or change
repository agent behavior in future sessions. Keep repository guidance
consistent with higher-priority instructions and actual permissions.

Use `python-docx` to generate independent DOCX fixtures and to open and inspect
basic features of existing Word files and edited outputs. Use package-level
checks for relationships, opaque markup, and exact byte preservation;
python-docx inspection does not replace those checks or the rocx editor.

When adding functionality, update both `examples/all_features.rs` (and its support
modules) and `examples/all_features.py`, plus `examples/ALL_FEATURES.md` and the
parity/coverage checks. Keep visible examples clearly labeled. Run exact DOCX
byte-parity verification; normalized XML equality is not a substitute.
