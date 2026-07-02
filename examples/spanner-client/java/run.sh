#!/usr/bin/env bash
# Run the Java example with JVM flags appropriate for the current JDK.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../../.." && pwd)"

# shellcheck source=maven-opts.sh
source "$ROOT/maven-opts.sh"

if [[ -z "${SPANNERPLAN_FFI_LIB:-}" ]]; then
  case "$(uname -s)" in
    Darwin) export SPANNERPLAN_FFI_LIB="$REPO/target/debug/libspannerplan_ffi.dylib" ;;
    Linux) export SPANNERPLAN_FFI_LIB="$REPO/target/debug/libspannerplan_ffi.so" ;;
    MINGW*|MSYS*|CYGWIN*) export SPANNERPLAN_FFI_LIB="$REPO/target/debug/spannerplan_ffi.dll" ;;
    *) echo "Set SPANNERPLAN_FFI_LIB for this platform." >&2; exit 1 ;;
  esac
fi

mvn -q -f "$REPO/bindings/java/pom.xml" install -DskipTests
cd "$ROOT"
mvn -q compile exec:java -Dexec.classpathScope=runtime -Dexec.args="$*"
