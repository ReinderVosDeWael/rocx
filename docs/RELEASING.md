# Releasing

Publish packages only through [the GitHub Actions workflow](../.github/workflows/publish.yml).
Creating a local tag does not publish anything. Pushing a `v*` tag to GitHub
starts CI, which validates, builds, and publishes to PyPI, crates.io and npm.
Normal workflow dispatches are preflight only, even when run on a tag: they
build artifacts without uploading them to the registries. The one-time npm
recovery below is an explicit exception. Do not publish packages with local
upload commands.

The release workflow runs the installed JavaScript consumer gate. The npm job
uploads the qualified tarball without rebuilding it. On the first release it
stages the tarball for approval rather than publishing it directly.

## One-time registry setup

Configure a GitHub environment named `pypi` and a pending PyPI trusted publisher
for project `rocx`:

| Field | Value |
| --- | --- |
| Owner | `ReinderVosDeWael` |
| Repository | `rocx` |
| Workflow | `publish.yml` |
| Environment | `pypi` |

No PyPI token is stored in GitHub. The publish job exchanges a GitHub OIDC token
for a short-lived PyPI credential.

crates.io trusted publishing uses the environment `crates-io` and the same
workflow filename. Version 0.3.0 used the bootstrap `CARGO_REGISTRY_TOKEN`
secret. Starting in 0.4.0 the workflow only uses OIDC. Verify the following
trusted-publisher configuration in crates.io, then remove the unused secret:

| Field | Value |
| --- | --- |
| Repository | `ReinderVosDeWael/rocx` |
| Workflow | `publish.yml` |
| Environment | `crates-io` |

Environment protection rules (for example required reviewers) are strongly
recommended for all publishing environments.

For `rocx-docx` on npm, use the GitHub environment `npm`:

- For a package that does not yet exist, create a short-lived npm granular
  token authorized to stage a new public package, and store it as the
  **environment secret** `NPM_BOOTSTRAP_TOKEN`. The tag run stages the
  qualified tarball. An npm maintainer must approve the package with 2FA.
  Inspect the staging state before rerunning a failed publication job.
- Once the package exists, remove the bootstrap secret and revoke the token.
  In [npm trusted publishing](https://docs.npmjs.com/trusted-publishers) authorize
  owner `ReinderVosDeWael`, repository `rocx`, workflow `publish.yml`,
  environment `npm`, with **npm publish** allowed. Configure the binding
  near the next release so it does not expire before its first use.

These tables specify the intended configuration, not a verification of private
registry settings. The [0.3.0 publish run](https://github.com/ReinderVosDeWael/rocx/actions/runs/45191346555)
verified PyPI OIDC at that commit, but skipped crates.io OIDC. Before pushing
any publishing tag, manually verify each trusted publisher's repository,
workflow and environment in its registry and confirm the corresponding GitHub
environment and credentials. A repository rename does not itself prove that
an existing publisher was updated.

## Release checklist

Before selecting a tag, verify the release candidate includes all intended source,
tests, documentation and license notices. Build the site locally using
[the documentation runbook](DOCUMENTATION.md) and review the support limits in
[COMPATIBILITY.md](../COMPATIBILITY.md). Do not use a tag to test packaging:
the tag push starts the publication workflow. Use an untagged validation run
first to exercise the hosted build matrix without publishing.

1. Update `Cargo.toml`, `pyproject.toml`, the Python runtime version
   (`rocx.__version__`), and `javascript/package.json` to the same version.
2. Refresh the root lock files with `cargo check` and `uv lock`. Refresh the
   JavaScript adapter's lock file too: its path dependency records the root
   crate version. With the `wasm32-unknown-unknown` target installed, run
   `cargo check --manifest-path javascript/Cargo.toml --target wasm32-unknown-unknown`.
3. Verify the tag matches every recorded version:

   ```sh
   python tools/check_release.py --tag v0.4.0
   ```

4. Run the verification runbook in [tests/README.md](../tests/README.md),
   including the required official-schema invocation and the isolated wheel
   probe. The schema command must set `ROCX_REQUIRE_SCHEMAS=1`; its successful
   completion proves missing schemas did not become skips.
5. Commit the release candidate. Run the hosted workflow manually from an
   untagged ref at that commit. Enable `verify_crates_oidc` to test the
   crates.io credential exchange independently of the bootstrap token.
   Confirm validation and artifact builds succeed without registry uploads.
6. Create and push the tag for the commit that passed preflight:

   ```sh
   git tag v0.4.0
   git push origin v0.4.0
   ```

The tag push runs the full release gate. Publishing starts only after validation
and every wheel/sdist build succeeds. Retry a failed tag-triggered release
using GitHub Actions' rerun controls for the original tag-push run, not a new
manual dispatch.

For the original `v0.4.0` tag only, the `recover_npm_040` dispatch on `main`
stages the already-qualified npm tarball from run `45623224278`. Use this
instead of rerunning the original job's incorrect local tarball path. This
does not upload to crates.io or PyPI or change the tag.

## Published artifacts

| Artifact | Platforms |
| --- | --- |
| Python wheel | Linux x86_64 (`manylinux_2_17`), macOS universal2, Windows x86_64 |
| Python sdist | Any platform with a Rust toolchain |
| Rust crate | `rocx` from crates.io |
| npm tarball | `rocx-docx` for Node and browsers |

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
