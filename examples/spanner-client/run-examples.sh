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

# --- optional-language pre-install (never abort the whole script) ---

RUBY_BUNDLE=""
PHP_READY=0
CPP_READY=0

resolve_bundle() {
  local gemfile_lock="$ROOT/ruby/Gemfile.lock"
  local want="" c ver maj want_maj candidates=() seen=""

  if [[ -f "$gemfile_lock" ]]; then
    want="$(awk '/^BUNDLED WITH$/{getline; print $1; exit}' "$gemfile_lock")"
  fi
  [[ -n "${BUNDLE_BIN:-}" ]] && candidates+=("$BUNDLE_BIN")
  if command -v bundle >/dev/null 2>&1; then
    candidates+=("$(command -v bundle)")
  fi
  candidates+=(/opt/homebrew/bin/bundle /usr/local/bin/bundle)

  [[ -n "$want" ]] && want_maj="${want%%.*}"
  for c in "${candidates[@]}"; do
    [[ -z "$c" || ! -x "$c" ]] && continue
    [[ "$seen" == *"|$c|"* ]] && continue
    seen+="|$c|"
    ver="$("$c" --version 2>/dev/null | awk '{print $NF}')" || continue
    [[ -z "$ver" ]] && continue
    if [[ -z "$want" || "$ver" == "$want" || "${ver%%.*}" == "$want_maj" ]]; then
      printf '%s\n' "$c"
      return 0
    fi
  done
  return 1
}

if [[ -f "$ROOT/ruby/Gemfile" ]]; then
  if RUBY_BUNDLE="$(resolve_bundle)"; then
    echo "Preparing Ruby bundle (once, before parallel runs)..." >&2
    if (
      cd "$ROOT/ruby"
      "$RUBY_BUNDLE" config set --local path vendor/bundle
      "$RUBY_BUNDLE" install --quiet
    ); then
      :
    else
      echo "Skipping Ruby (bundle install failed; need Bundler $(awk '/^BUNDLED WITH$/{getline; print $1; exit}' "$ROOT/ruby/Gemfile.lock" 2>/dev/null || echo '?') on PATH — see README)." >&2
      RUBY_BUNDLE=""
    fi
  else
    echo "Skipping Ruby (no compatible bundle found; Gemfile.lock expects Bundler $(awk '/^BUNDLED WITH$/{getline; print $1; exit}' "$ROOT/ruby/Gemfile.lock" 2>/dev/null || echo '?'))." >&2
  fi
fi

if command -v composer >/dev/null && [[ -f "$ROOT/php/composer.json" ]]; then
  echo "Preparing PHP composer deps (once, before parallel runs)..." >&2
  if (cd "$ROOT/php" && composer install --quiet --no-interaction --ignore-platform-req=ext-grpc); then
    PHP_READY=1
  else
    echo "Skipping PHP (composer install failed; see php/README or run composer install manually)." >&2
  fi
fi

if command -v cmake >/dev/null && [[ -f "$ROOT/cpp/CMakeLists.txt" ]]; then
  echo "Building C++ example (once, before parallel runs)..." >&2
  CPP_BUILD="$ROOT/cpp/build"
  CMAKE_ARGS=(-S "$ROOT/cpp" -B "$CPP_BUILD")
  if toolchain="$("$ROOT/vcpkg-detect.sh" 2>/dev/null)"; then
    export CMAKE_TOOLCHAIN_FILE="$toolchain"
    CMAKE_ARGS+=(-DCMAKE_TOOLCHAIN_FILE="$CMAKE_TOOLCHAIN_FILE")
    echo "Using vcpkg toolchain: $CMAKE_TOOLCHAIN_FILE" >&2
  fi
  if cmake "${CMAKE_ARGS[@]}" >/dev/null 2>&1; then
    if cmake --build "$CPP_BUILD" --target analyze_and_render -j >/dev/null 2>&1 \
      && [[ -x "$CPP_BUILD/analyze_and_render" ]]; then
      CPP_READY=1
    else
      echo "Skipping C++ (build failed; google-cloud-cpp may need vcpkg — see README)." >&2
    fi
  else
    echo "Skipping C++ (cmake configure failed; install vcpkg or google-cloud-cpp — see README)." >&2
  fi
fi

# --- core-language pre-install (still fatal on failure) ---

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
LABELS=()
RESULTS=()
OK=0
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
  LABELS+=("$label")
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

  if [[ "$CPP_READY" -eq 1 ]]; then
    run_bg "cpp/$mode" bash -lc "
      cd '$ROOT/cpp'
      '$ROOT/cpp/build/analyze_and_render' --query-mode '$mode'
    "
  fi

  if [[ -n "$RUBY_BUNDLE" ]]; then
    RUBY_BIN_DIR="$(cd "$(dirname "$RUBY_BUNDLE")" && pwd)"
    run_bg "ruby/$mode" bash -c "
      export PATH='$RUBY_BIN_DIR':\"\$PATH\"
      cd '$ROOT/ruby'
      '$RUBY_BUNDLE' exec ruby analyze_and_render.rb --query-mode '$mode'
    "
  fi

  if [[ "$PHP_READY" -eq 1 ]] && command -v php >/dev/null; then
    run_bg "php/$mode" bash -lc "
      cd '$ROOT/php'
      php -d ffi.enable=true analyze_and_render.php --query-mode '$mode'
    "
  fi
done

for i in "${!PIDS[@]}"; do
  if wait "${PIDS[$i]}"; then
    RESULTS[$i]="ok"
    ((OK++)) || true
  else
    RESULTS[$i]="fail"
    ((FAIL++)) || true
  fi
done

lang_summary_status() {
  local lang="$1" saw=0 all_ok=1 i
  for i in "${!LABELS[@]}"; do
    [[ "${LABELS[$i]%%/*}" == "$lang" ]] || continue
    saw=1
    [[ "${RESULTS[$i]}" == "ok" ]] || all_ok=0
  done
  if [[ "$saw" -eq 1 ]]; then
    [[ "$all_ok" -eq 1 ]] && echo OK || echo FAIL
    return
  fi
  case "$lang" in
    cpp) [[ "$CPP_READY" -eq 0 ]] && { echo SKIPPED; return; } ;;
    ruby) [[ -z "$RUBY_BUNDLE" ]] && { echo SKIPPED; return; } ;;
    php) [[ "$PHP_READY" -eq 0 ]] && { echo SKIPPED; return; } ;;
  esac
  echo SKIPPED
}

echo ""
echo "=== Example summary ==="
for lang in python java node dotnet go rust cpp ruby php; do
  status="$(lang_summary_status "$lang")"
  printf '  %-8s %s\n' "$lang" "$status"
done
echo "  total: $OK ok, $FAIL fail"

if [[ "$OK" -eq 0 && "$FAIL" -gt 0 ]]; then
  exit 1
fi
exit 0
