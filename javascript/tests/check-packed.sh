#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "$0")/../.." && pwd)
output=$(mktemp -d "${TMPDIR:-/tmp}/rocx-packed-js.XXXXXX")
echo "Packed artifact, consumers and DOCX outputs: $output"
image="$root/tests/fixtures/images/sample.png"
examples="$root/examples/javascript"

npm pack "$root/javascript" --silent --pack-destination "$output" > "$output/package-name"
artifact="$output/$(cat "$output/package-name")"
mkdir "$output/node" "$output/web"
cp "$examples"/{author.mjs,node.mjs} "$root/javascript/tests/node-errors.mjs" "$output/node/"
cp "$examples"/{author.mjs,browser.mjs,index.html,vite.config.mjs} "$output/web/"
cp "$root/javascript/tests"/{package.json,package-lock.json,types.mts,browser-test.mjs} "$output/web/"

(
    cd "$output/node"
    printf '{ "private": true }\n' > package.json
    npm install --ignore-scripts --offline --no-audit --no-fund --no-save "$artifact"
    test -f node_modules/rocx-docx/LICENSE
    tar -tzf node_modules/rocx-docx/rocx-source.tar.gz > source-files.txt
    grep -Fx src/lib.rs source-files.txt
    grep -Fx javascript/src/lib.rs source-files.txt
    node node-errors.mjs
    node node.mjs "$image" "$output/node.docx"
)
(
    cd "$output/web"
    npm ci --no-audit --no-fund
    # npm may look up registry metadata for the Vite toolchain during reification.
    # The tarball itself still has no dependencies and installs offline above.
    npm install --ignore-scripts --prefer-offline --no-audit --no-fund --no-save "$artifact"
    if [[ "${ROCX_INSTALL_BROWSER:-0}" == 1 ]]; then
        ./node_modules/.bin/playwright install --with-deps chromium
    fi
    ./node_modules/.bin/tsc --noEmit --strict --target es2022 --module NodeNext types.mts
    ./node_modules/.bin/tsc --noEmit --strict --target es2022 --module esnext --moduleResolution bundler types.mts
    node browser-test.mjs "$image" "$output"
)
cd "$root"
cargo run --locked --example javascript_workflow -- "$output/rust.docx"
uv run --no-sync python examples/javascript_workflow.py "$output/python.docx"
uv run --no-sync python tools/check_javascript.py "$output"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    printf 'tarball=%s\n' "$artifact" >> "$GITHUB_OUTPUT"
fi
