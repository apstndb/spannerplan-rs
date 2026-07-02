#!/usr/bin/env bash
# Build @spannerplan/core WASM artifacts (bundler + nodejs targets).
# Delegates to the canonical script under js/packages/spannerplan/.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec bash "$ROOT/js/packages/spannerplan/scripts/build-wasm.sh"
