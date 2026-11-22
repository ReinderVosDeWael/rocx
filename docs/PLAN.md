# Active plan — 0.3.0 release preparation

rocx is feature-complete for its current support profile, not for all OOXML.
Keep the feature freeze in place. The backlog below is not a release blocker.

## Release checklist

1. [Documentation audit](DOCUMENTATION-AUDIT.md) applied and verified locally.
2. Review the release candidate for packaging completeness, including license
   notices, release notes, and whether internal planning documents should ship.
   Keep unrelated working-tree changes intact.
3. Follow the [release runbook](RELEASING.md) for `0.3.0`. Confirm the hosted
   build matrix and publication destinations before tagging.
4. Commit, push, deploy, or publish only with authorization for that action.
   Only GitHub Actions publishes packages, on a pushed `v*` tag after validation
   and artifact builds. Manual dispatch is preflight only, even on tagged refs.

## Release candidate status

- The Rust and Python manifests, lockfiles, and Python runtime report `0.3.0`.
- The recorded local baseline passed 269 Rust tests, three doctests, 741
  schema-required Python tests, and Clippy. Both final catalogs validated 20
  covered parts plus OPC relationships; final and template byte parity passed.
- After the audit edits, the strict site build checked 477 HTML pages and
  verified exact catalog parity. All 11 documentation tests passed, and the
  standalone Rust and Python quickstart and template snippets ran locally.
- The earlier version gate, offline lock check, Cargo packaging and isolated
  CPython 3.12 wheel probe also passed. Cargo packaging used `--allow-dirty`
  for local preflight.
- Hosted release preflight has not run. No tag was created or pushed, and no
  deployment or publication was performed.

## Deferred backlog

- Workbook-free clustered multi-series charts and a literal superscript
  prototype.
- Relationship-based validation for relocated font tables and more OPC
  relationship graph tests.
- An internal `AlternateContent` selector, subject to the
  [proposed editor contract](ALTERNATE-CONTENT-DESIGN.md).
- A glossary preservation prototype.

See the [archived roadmap and handoffs](ROADMAP-HISTORY.md) for earlier
decisions, checkpoint evidence, and prototype notes. Instructions there are
historical, not current assignments or authorization.
