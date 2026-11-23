#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")"

if [[ "$(wasm-bindgen --version 2>/dev/null || true)" != "wasm-bindgen 0.2.130" ]]; then
    echo "Install the matching CLI: cargo install wasm-bindgen-cli --version 0.2.130 --locked" >&2
    exit 1
fi

cargo build --locked --manifest-path Cargo.toml --release \
    --target wasm32-unknown-unknown --target-dir target
wasm-bindgen --target web --no-typescript --out-dir pkg --out-name rocx \
    target/wasm32-unknown-unknown/release/rocx_js.wasm

# Distribute the exact rocx source and the interface/build files with the Wasm.
cp ../LICENSE LICENSE
tar -czf rocx-source.tar.gz -C .. \
    LICENSE README.md Cargo.toml Cargo.lock src assets \
    examples/javascript tests/fixtures/images/sample.png \
    javascript/{Cargo.toml,Cargo.lock,build.sh,package.json,README.md,index.d.ts,browser.js,node.js,generate.js,src}
