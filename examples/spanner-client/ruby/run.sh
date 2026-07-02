#!/usr/bin/env bash
# Run the Ruby example (bundle install is done once; safe for repeated use).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../../.." && pwd)"

if [[ -z "${SPANNERPLAN_FFI_LIB:-}" ]]; then
  case "$(uname -s)" in
    Darwin) export SPANNERPLAN_FFI_LIB="$REPO/target/debug/libspannerplan_ffi.dylib" ;;
    Linux) export SPANNERPLAN_FFI_LIB="$REPO/target/debug/libspannerplan_ffi.so" ;;
    MINGW*|MSYS*|CYGWIN*) export SPANNERPLAN_FFI_LIB="$REPO/target/debug/spannerplan_ffi.dll" ;;
    *) echo "Set SPANNERPLAN_FFI_LIB for this platform." >&2; exit 1 ;;
  esac
fi

cd "$ROOT"
BUNDLE_BIN="${BUNDLE_BIN:-$(command -v bundle 2>/dev/null || true)}"
if [[ -z "$BUNDLE_BIN" && -x /opt/homebrew/bin/bundle ]]; then
  BUNDLE_BIN=/opt/homebrew/bin/bundle
fi
if [[ -z "$BUNDLE_BIN" ]]; then
  echo "bundle not found (need Bundler matching Gemfile.lock — see README)." >&2
  exit 1
fi
export PATH="$(cd "$(dirname "$BUNDLE_BIN")" && pwd):${PATH}"
"$BUNDLE_BIN" config set --local path vendor/bundle
"$BUNDLE_BIN" install --quiet
"$BUNDLE_BIN" exec ruby analyze_and_render.rb "$@"
