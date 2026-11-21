# Releasing

Publishing is tag-gated. `.github/workflows/publish.yml` never publishes from a
branch `workflow_dispatch`; that mode validates and builds the release artifacts
without uploading them.

## One-time registry setup

Configure a GitHub environment named `pypi` and a pending PyPI trusted publisher
for project `rocx`:

| Field | Value |
| --- | --- |
| Owner | `ReinderVosDeWael` |
| Repository | `rv-docx-rs` |
| Workflow | `publish.yml` |
| Environment | `pypi` |

No PyPI token is stored in GitHub. The publish job exchanges a GitHub OIDC token
for a short-lived PyPI credential.

crates.io trusted publishing uses the environment `crates-io` and the same
workflow filename. Because `rocx` is not yet on crates.io, the first release
needs a repository or `crates-io` environment secret named
`CARGO_REGISTRY_TOKEN`. The workflow uses that secret when present; otherwise it
authenticates through `rust-lang/crates-io-auth-action`. After the first release,
configure the crates.io trusted publisher and delete the secret:

| Field | Value |
| --- | --- |
| Repository | `ReinderVosDeWael/rv-docx-rs` |
| Workflow | `publish.yml` |
| Environment | `crates-io` |

Environment protection rules (for example required reviewers) are strongly
recommended for both environments.

## Release checklist

1. Update the version in `Cargo.toml` and `pyproject.toml`. The versions are
   independent fields and must remain identical.
2. Refresh both lock files with `cargo check` and `uv lock`.
3. Verify the tag matches every recorded version:

   ```sh
   python tools/check_release.py --tag v0.2.1
   ```

4. Run the verification runbook in [tests/README.md](../tests/README.md),
   including the required official-schema run and the isolated wheel probe.
5. Commit the version and lock-file changes, then create and push the tag:

   ```sh
   git tag v0.2.1
   git push origin v0.2.1
   ```

The tag push runs the full release gate. Publishing starts only after validation
and every wheel/sdist build succeeds.

## Published artifacts

| Artifact | Platforms |
| --- | --- |
| Python wheel | Linux x86_64 (`manylinux_2_17`), macOS universal2, Windows x86_64 |
| Python sdist | Any platform with a Rust toolchain |
| Rust crate | `rocx` from crates.io |

Wheels target CPython 3.12 and newer through the `abi3-py312` contract. Other
architectures can install from the sdist; they are not currently built as
wheels. Extending the matrix is a deliberate support decision, not an automatic
consequence of adding a target.

## Reruns and failures

PyPI uploads use `skip-existing`, so a partially completed upload can be retried.
crates.io versions are immutable; a failed or incorrect crate release requires a
new version. Never move or reuse a published tag: bump the version, commit, and
tag the new commit.

The validation job downloads the digest-pinned official schemas (or reuses the
CI cache) and runs with `ROCX_REQUIRE_SCHEMAS=1`, so missing standards cannot
silently turn into skips. It also runs `cargo publish --dry-run --locked`, which
checks the exact crate package without uploading it.
