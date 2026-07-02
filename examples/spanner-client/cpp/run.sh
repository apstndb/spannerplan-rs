#!/usr/bin/env bash
# Build and run the C++ Spanner client example.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../../.." && pwd)"
EXAMPLES_ROOT="$(cd "$ROOT/.." && pwd)"
BUILD="$ROOT/build"

if [[ -z "${SPANNERPLAN_FFI_LIB:-}" ]]; then
  case "$(uname -s)" in
    Darwin) export SPANNERPLAN_FFI_LIB="$REPO/target/debug/libspannerplan_ffi.dylib" ;;
    Linux) export SPANNERPLAN_FFI_LIB="$REPO/target/debug/libspannerplan_ffi.so" ;;
    MINGW*|MSYS*|CYGWIN*) export SPANNERPLAN_FFI_LIB="$REPO/target/debug/spannerplan_ffi.dll" ;;
    *) echo "Set SPANNERPLAN_FFI_LIB for this platform." >&2; exit 1 ;;
  esac
fi

if [[ ! -f "$SPANNERPLAN_FFI_LIB" ]]; then
  echo "Building spannerplan-ffi..." >&2
  (cd "$REPO" && cargo build -p spannerplan-ffi)
fi

CMAKE_ARGS=(-S "$ROOT" -B "$BUILD")
if toolchain="$("$EXAMPLES_ROOT/vcpkg-detect.sh" 2>/dev/null)"; then
  export CMAKE_TOOLCHAIN_FILE="$toolchain"
  CMAKE_ARGS+=(-DCMAKE_TOOLCHAIN_FILE="$CMAKE_TOOLCHAIN_FILE")
  echo "Using vcpkg toolchain: $CMAKE_TOOLCHAIN_FILE" >&2
elif [[ -n "${CMAKE_TOOLCHAIN_FILE:-}" ]]; then
  echo "CMAKE_TOOLCHAIN_FILE is set but not found: $CMAKE_TOOLCHAIN_FILE" >&2
  exit 1
else
  echo "vcpkg not found; trying system google-cloud-cpp (install vcpkg for manifest build)." >&2
fi

if ! cmake "${CMAKE_ARGS[@]}"; then
  cat >&2 <<'EOF'
CMake configure failed. Install google-cloud-cpp via vcpkg:

  git clone https://github.com/microsoft/vcpkg.git ~/vcpkg
  ~/vcpkg/bootstrap-vcpkg.sh
  export VCPKG_ROOT=~/vcpkg
  cd examples/spanner-client/cpp && ./run.sh

See examples/spanner-client/README.md for details.
EOF
  exit 1
fi

cmake --build "$BUILD" --target analyze_and_render
"$BUILD/analyze_and_render" "$@"
