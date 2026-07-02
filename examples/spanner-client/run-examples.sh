#!/usr/bin/env bash
# Run all Spanner client examples (PLAN + PROFILE) in parallel where possible.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../.." && pwd)"

if [[ -z "${SPANNER_PROJECT_ID:-}" || -z "${SPANNER_INSTANCE_ID:-}" || -z "${SPANNER_DATABASE_ID:-}" ]]; then
  echo "Set SPANNER_PROJECT_ID, SPANNER_INSTANCE_ID, and SPANNER_DATABASE_ID." >&2
  exit 1
fi

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

if command -v dotnet >/dev/null; then
  echo "Building dotnet example (once, before parallel runs)..." >&2
  (cd "$ROOT/dotnet" && dotnet build -v q -p:UseSharedCompilation=false)
fi

if command -v python3 >/dev/null; then
  echo "Preparing Python venv (once, before parallel runs)..." >&2
  (
    cd "$ROOT/python"
    if [[ ! -d .venv ]]; then
      python3 -m venv .venv
    fi
    # shellcheck source=/dev/null
    source .venv/bin/activate
    pip install -q -r requirements.txt
    pip install -q -e "$REPO/bindings/python"
  )
fi

MODES=(PLAN PROFILE)
PIDS=()
FAIL=0

run_bg() {
  local label="$1"
  shift
  local log
  log="$(mktemp "${TMPDIR:-/tmp}/spanner-example.XXXXXX")"
  echo "==> $label (log: $log)"
  (
    set +e
    "$@" >"$log" 2>&1
    ec=$?
    if [[ $ec -eq 0 ]]; then
      echo "OK  $label"
      head -n 8 "$log" | sed "s/^/    /"
    else
      echo "FAIL $label (exit $ec)" >&2
      cat "$log" >&2
      exit "$ec"
    fi
    rm -f "$log"
  ) &
  PIDS+=($!)
}

for mode in "${MODES[@]}"; do
  export SPANNER_QUERY_MODE="$mode"

  if command -v python3 >/dev/null; then
    run_bg "python/$mode" bash -lc "
      cd '$ROOT/python'
      source .venv/bin/activate
      python analyze_and_render.py --query-mode '$mode'
    "
  fi

  if command -v mvn >/dev/null; then
    run_bg "java/$mode" bash -lc "
      source '$ROOT/java/maven-opts.sh'
      mvn -q -f '$REPO/bindings/java/pom.xml' install -DskipTests
      cd '$ROOT/java'
      mvn -q compile exec:java -Dexec.classpathScope=runtime \
        -Dexec.args='--query-mode $mode'
    "
  fi

  if command -v node >/dev/null; then
    run_bg "node/$mode" bash -lc "
      SPANNERPLAN_QUIET_WASM_BUILD=1 npm run build -w @spannerplan/core --prefix '$REPO/js' >/dev/null 2>&1
      cd '$ROOT/node'
      npm install --silent
      node analyze_and_render.mjs --query-mode '$mode'
    "
  fi

  if command -v dotnet >/dev/null; then
    run_bg "dotnet/$mode" bash -lc "
      cd '$ROOT/dotnet'
      dotnet run --no-build -- --query-mode '$mode'
    "
  fi

  if command -v go >/dev/null; then
    run_bg "go/$mode" bash -lc "cd '$ROOT/go' && go run . --query-mode '$mode'"
  fi

  if command -v cargo >/dev/null; then
    run_bg "rust/$mode" cargo run --manifest-path "$ROOT/rust/Cargo.toml" --quiet -- --query-mode "$mode"
  fi
done

for pid in "${PIDS[@]}"; do
  if ! wait "$pid"; then
    FAIL=1
  fi
done

exit "$FAIL"
