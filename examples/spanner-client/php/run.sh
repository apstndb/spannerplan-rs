#!/usr/bin/env bash
# Run the PHP example (composer install is done once; safe for repeated use).
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
composer install --quiet --no-interaction --ignore-platform-req=ext-grpc
php -d ffi.enable=true analyze_and_render.php "$@"
