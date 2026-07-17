#!/usr/bin/env bash
# Canonical wasm-pack build for @spannerplan/core.
# Also invokable from repo root: ./scripts/build-wasm.sh
#
# Two artifacts:
#   wasm/       — slim core (wire + JSON renderer; YAML parsed in JS) for browsers
#   wasm-node/  — full (yaml + wire + cli) for Node.js / rendertree CLI
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
WASM_CRATE="$ROOT/crates/spannerplan-wasm"
OUT="$ROOT/js/packages/spannerplan"

WASM_PACK="${WASM_PACK:-wasm-pack}"
if ! command -v "$WASM_PACK" >/dev/null 2>&1; then
  WASM_PACK="$HOME/.cargo/bin/wasm-pack"
fi

# wasm-pack defaults to --log-level info. Set SPANNERPLAN_QUIET_WASM_BUILD=1 or
# WASM_PACK_LOG_LEVEL=error to silence [INFO] lines (e.g. run-examples.sh).
WASM_PACK_LOG_LEVEL="${WASM_PACK_LOG_LEVEL:-info}"
if [[ "${SPANNERPLAN_QUIET_WASM_BUILD:-}" == "1" ]]; then
  WASM_PACK_LOG_LEVEL=error
fi
WASM_PACK_ARGS=(--log-level "$WASM_PACK_LOG_LEVEL")
CARGO_QUIET=()
if [[ "$WASM_PACK_LOG_LEVEL" == "error" ]]; then
  CARGO_QUIET=(--quiet)
fi

build_wasm() {
  local target="$1"
  local out_dir="$2"
  local features="$3"
  if ((${#CARGO_QUIET[@]})); then
    "$WASM_PACK" "${WASM_PACK_ARGS[@]}" build --release --target "$target" --out-dir "$out_dir" \
      -- --no-default-features --features "$features" "${CARGO_QUIET[@]}"
  else
    "$WASM_PACK" "${WASM_PACK_ARGS[@]}" build --release --target "$target" --out-dir "$out_dir" \
      -- --no-default-features --features "$features"
  fi
}

cd "$WASM_CRATE"

# wasm-pack warns when Cargo.toml has license but the crate dir has no LICENSE file.
if [[ ! -e LICENSE && -f "$ROOT/LICENSE" ]]; then
  ln -sf "$ROOT/LICENSE" LICENSE
fi

# Browser: the web target uses a package-relative `new URL(..., import.meta.url)`
# initializer. Vite and other modern bundlers turn that URL into a deployed
# asset; wasm-pack's bundler target instead relies on the still-unsupported
# WebAssembly ESM integration proposal.
build_wasm web "$OUT/wasm" wire

# Node: full feature set for YAML stdin, wire bytes, and rendertree CLI parity.
build_wasm nodejs "$OUT/wasm-node" yaml,wire,cli

# wasm-pack writes .gitignore with "*" in each out dir; npm pack honors it and
# would omit the .wasm binaries from release tarballs.
rm -f "$OUT/wasm/.gitignore" "$OUT/wasm-node/.gitignore"
