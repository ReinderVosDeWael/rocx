Before doing anything, consult `docs/PLAN.md`. Update that plan when your work
or the engineer's direction changes it.

Never start or resume an Astra subagent unless the user explicitly authorizes
Astra for that task. General permission to delegate or parallelize is not
authorization to use Astra. Otherwise, explicitly select Sol for difficult
subagent work and Terra for easy work so a default cannot select Astra.

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

Reuse the checkout's Cargo `target/` for routine builds and tests. Do not
create a persistent `CARGO_TARGET_DIR` per issue, PR, or test run. When a
qualification check requires isolation, use one run-owned scratch directory
and clean it up on success, failure, and interruption. Retain evidence only
when needed, and record its location and cleanup plan.

Use `python-docx` to generate independent DOCX fixtures and to open and inspect
basic features of existing Word files and edited outputs. Use package-level
checks for relationships, opaque markup, and exact byte preservation;
python-docx inspection does not replace those checks or the rocx editor.

Keep Rust, Python and JavaScript at feature parity. When adding functionality,
update all three public APIs (including JavaScript declarations), the Rust,
Python and JavaScript all-features catalogs, `examples/ALL_FEATURES.md`, and
the parity/coverage checks. Document all three spellings together. Keep samples
clearly labeled. Compare full DOCX bytes across bindings and test inspection
results, errors and anchor behavior separately in installed consumers.
