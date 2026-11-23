#!/usr/bin/env bash
set -euo pipefail

fixtures=$(cd -- "$(dirname -- "$0")" && pwd)
root=$(cd -- "$fixtures/../../.." && pwd)
cd -- "${1:?pass the installed web consumer directory from check-packed.sh}"
test -f node_modules/rocx-docx/pkg/rocx_bg.wasm
cp -- "$fixtures"/b0*.mjs "$fixtures"/b0.mts .
node b0.mjs "$root/tests/fixtures/images/sample.png"
node b0-retry.mjs
./node_modules/.bin/tsc --noEmit --strict --target es2022 --module NodeNext b0.mts
./node_modules/.bin/tsc --noEmit --strict --target es2022 --module esnext --moduleResolution bundler b0.mts
node b0-browser.mjs
