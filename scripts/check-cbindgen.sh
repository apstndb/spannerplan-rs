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

cbindgen \
  --crate "$CRATE_DIR" \
  --lang c \
  --guard SPANNERPLAN_H \
  --output "$TMP"

if ! diff -u "$COMMITTED" "$TMP"; then
  echo "error: spannerplan.h is out of date; regenerate with:" >&2
  echo "  cbindgen --crate crates/spannerplan-ffi --lang c --guard SPANNERPLAN_H --output crates/spannerplan-ffi/spannerplan.h" >&2
  exit 1
fi

echo "cbindgen header check ok"
