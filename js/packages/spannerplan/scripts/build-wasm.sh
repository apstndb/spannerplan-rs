#!/usr/bin/env bash
# Canonical wasm-pack build for @spannerplan/core.
# Also invokable from repo root: ./scripts/build-wasm.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
WASM_CRATE="$ROOT/crates/spannerplan-wasm"
OUT="$ROOT/js/packages/spannerplan"

WASM_PACK="${WASM_PACK:-wasm-pack}"
if ! command -v "$WASM_PACK" >/dev/null 2>&1; then
  WASM_PACK="$HOME/.cargo/bin/wasm-pack"
fi

cd "$WASM_CRATE"
"$WASM_PACK" build --target bundler --out-dir "$OUT/wasm"
"$WASM_PACK" build --target nodejs --out-dir "$OUT/wasm-node"
