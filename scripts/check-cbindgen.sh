#!/usr/bin/env bash
# Verify committed crates/spannerplan-ffi/spannerplan.h matches cbindgen output.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$ROOT/crates/spannerplan-ffi"
COMMITTED="$CRATE_DIR/spannerplan.h"
TMP="$(mktemp)"

cleanup() {
  rm -f "$TMP"
}
trap cleanup EXIT

if ! command -v cbindgen >/dev/null 2>&1; then
  echo "cbindgen not found on PATH; installing cbindgen 0.29.4..." >&2
  cargo install cbindgen --version 0.29.4 --locked --quiet
fi

CBINDGEN="${CBINDGEN:-$(command -v cbindgen || echo "${CARGO_HOME:-$HOME/.cargo}/bin/cbindgen")}"

"$CBINDGEN" \
  "$CRATE_DIR" \
  --config "$CRATE_DIR/cbindgen.toml" \
  --output "$TMP"

if ! diff -u "$COMMITTED" "$TMP"; then
  echo "error: spannerplan.h is out of date; regenerate with:" >&2
  echo "  cbindgen crates/spannerplan-ffi --config crates/spannerplan-ffi/cbindgen.toml --output crates/spannerplan-ffi/spannerplan.h" >&2
  exit 1
fi

echo "cbindgen header check ok"
