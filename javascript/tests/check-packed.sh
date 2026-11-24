#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "$0")/../.." && pwd)
output=$(mktemp -d "${TMPDIR:-/var/tmp}/rocx-packed-js.XXXXXX")
echo "Packed artifact, consumers and DOCX outputs: $output"
image="$root/tests/fixtures/images/sample.png"
images=$(dirname -- "$image")
examples="$root/examples/javascript"
editor="$root/tests/fixtures/imported-editor.docx"
parity="$output/parity"
oracles="$parity/rust"
fixtures="$output/web/public/fixtures"
cd "$root"
echo "Packed consumer runtime: $(node --version)"

export PYTHON
PYTHON=$(uv run --no-sync python -c 'import sys; print(sys.executable)')
provenance="$root/javascript/tests/parity/integration/provenance.py"
"$PYTHON" "$provenance" pack "$output" --checkout "$root"
artifact="$output/$(cat "$output/package-name")"
# Keep native oracle builds outside the checkout, independently of the two
# unoverridden Wasm builds used to verify the packed artifact.
export CARGO_TARGET_DIR="${ROCX_NATIVE_TARGET_DIR:-$output/native-target}"
mkdir -p "$output/node" "$fixtures/editor" "$oracles/editor" "$parity/python"
cp "$examples"/{author.mjs,node.mjs,plain.mjs} \
    "$root/javascript/tests"/{node-errors,init-retry,unicode,cross-realm,diagnostics}.mjs "$output/node/"
cp "$examples"/{author.mjs,browser.mjs,index.html,vite.config.mjs} "$output/web/"
cp "$root/javascript/tests"/{package.json,package-lock.json,types.mts,contract-input.mts,input-contract.mjs,browser-test.mjs} "$output/web/"
cp -R "$root/javascript/tests/parity" "$output/web/"
cp "$images"/* "$editor" "$fixtures/"
uv run --no-sync python tests/javascript_parity/generate_fixtures.py "$fixtures"
uv run --no-sync python javascript/tests/parity/editor.py generate "$fixtures/editor"
for mode in rust node browser-dev browser; do
    mkdir -p "$parity/$mode/editor"
    cp "$fixtures/editor"/{base,control-source,relocated}.docx "$fixtures/editor/image.png" "$parity/$mode/editor/"
done
cargo run --quiet --locked --example javascript_parity_oracles -- "$oracles"
cargo run --quiet --locked --example javascript_editor_oracle -- "$oracles/editor"

# JSON is an unstable Rustdoc output format, not an unstable crate feature.
# Scope the opt-in to this documentation command on CI's stable toolchain.
RUSTC_BOOTSTRAP=rocx cargo rustdoc --locked --lib --target-dir "$output/inventory-target" \
    -- -Z unstable-options --output-format json

(
    cd "$output/node"
    printf '{ "private": true }\n' > package.json
    npm install --ignore-scripts --offline --no-audit --no-fund --no-save "$artifact"
    "$PYTHON" "$provenance" check-install "$output" --consumer node
    test -f node_modules/rocx-docx/LICENSE
    tar -tzf node_modules/rocx-docx/rocx-source.tar.gz > source-files.txt
    grep -Fx src/lib.rs source-files.txt
    grep -Fx javascript/src/lib.rs source-files.txt
    node node-errors.mjs
    node init-retry.mjs
    node unicode.mjs "$image"
    node cross-realm.mjs "$image"
    node diagnostics.mjs
    node node.mjs "$image" "$output/node.docx"
    node plain.mjs "$parity/node/plain.docx"
)
(
    cd "$output/web"
    npm ci --no-audit --no-fund
    # npm may look up registry metadata for the Vite toolchain during reification.
    # The tarball itself still has no dependencies and installs offline above.
    npm install --ignore-scripts --prefer-offline --no-audit --no-fund --no-save "$artifact"
    "$PYTHON" "$provenance" check-install "$output" --consumer web
    node parity/check-authoring-inventory.mjs "$output/inventory-target/doc/rocx.json" \
        node_modules/rocx-docx/index.d.ts node_modules/typescript/lib/typescript.js \
        | tee "$output/authoring-inventory.txt"
    if [[ "${ROCX_INSTALL_BROWSER:-0}" == 1 ]]; then
        ./node_modules/.bin/playwright install --with-deps chromium
    fi
    tsc=./node_modules/.bin/tsc
    portable_types=(types.mts contract-input.mts parity/{b0,editor,inspection,resolution,structured,styles,tables,text-layout}.mts
        parity/integration/{declarative,structured}.mts)
    node_types=(parity/{drawings,template}.mts)
    "$tsc" --noEmitOnError --strict --target es2022 --module NodeNext types.mts contract-input.mts
    "$tsc" --noEmit --strict --target es2022 --module NodeNext "${portable_types[@]}" "${node_types[@]}"
    "$tsc" --noEmit --strict --target es2022 --module esnext --moduleResolution bundler "${portable_types[@]}"
    # These fixtures import the intentionally Node-only conditional subpath.
    "$tsc" --noEmit --strict --target es2022 --module esnext --moduleResolution bundler \
        --customConditions node "${node_types[@]}"
    echo "Installed declarations: strict NodeNext and bundler checks passed"
    node input-contract.mjs "$image"
    node parity/b0.mjs "$image"
    node parity/b0-retry.mjs
    node parity/b0-browser.mjs
    node parity/inspection.mjs --browser
    node parity/resolution.mjs
    node parity/drawings.mjs "$images" "$oracles/drawings.docx" "$oracles/pictures.docx" "$oracles/image-path.docx"
    node parity/structured.mjs "$image" "$oracles/structured.docx"
    node parity/tables.mjs "--rust=$oracles/tables.docx"
    node parity/template.mjs "$root/assets/default.docx" "$image" "$editor" "$oracles"

    cp -R "$examples"/catalog "$examples/catalog.mjs" .
    node parity/integration/node.mjs "$fixtures" "$parity/node"
    node browser-test.mjs "$image" "$output"
)
for mode in node browser-dev browser; do
    echo "$mode: verifying editor readback and exact untouched payloads"
    uv run --no-sync python javascript/tests/parity/editor.py verify "$parity/$mode/editor"
done
cargo run --locked --example javascript_workflow -- "$output/rust.docx"
uv run --no-sync python examples/javascript_workflow.py "$output/python.docx"
cargo run --locked --example plain -- "$oracles/plain.docx"
uv run --no-sync python examples/plain.py "$parity/python/plain.docx"
uv run --no-sync python tools/check_examples.py --output-dir "$parity/catalog-references"
uv run --no-sync python tools/check_javascript.py "$output"
unset CARGO_TARGET_DIR
"$PYTHON" "$provenance" finish "$output" --checkout "$root"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    printf 'tarball=%s\n' "$artifact" >> "$GITHUB_OUTPUT"
    printf 'provenance=%s\n' "$output/provenance.json" >> "$GITHUB_OUTPUT"
fi
