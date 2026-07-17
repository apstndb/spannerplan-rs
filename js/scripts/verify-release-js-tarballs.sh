#!/usr/bin/env bash
# Verify that the release packages can be installed in clean consumers. The
# browser consumer is built with Vite and exercised in a real headless browser
# so the browser export, dynamic WASM load, and structured Plantree contract are
# covered by the release gate.
set -euo pipefail

JS_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$JS_ROOT/.." && pwd)"
WORK="$(mktemp -d)"
server_pid=""
browser_pid=""
stop_server() {
  local pid="$server_pid"
  server_pid=""
  if [[ -n "$pid" ]]; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
}

stop_browser() {
  local pid="$browser_pid"
  browser_pid=""
  if [[ -n "$pid" ]]; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
}

print_browser_smoke_failure() {
  local label="$1"
  local reason="$2"
  local chrome_log="$3"
  local server_log="$4"
  local result_file="$5"

  {
    echo "browser smoke failed (${label}): ${reason}"
    echo "--- Chrome stderr (${label}) ---"
    if [[ -s "$chrome_log" ]]; then
      cat "$chrome_log"
    else
      echo "(empty)"
    fi
    echo "--- smoke server log (${label}) ---"
    if [[ -s "$server_log" ]]; then
      cat "$server_log"
    else
      echo "(empty)"
    fi
    echo "--- page result JSON (${label}) ---"
    if [[ -f "$result_file" ]]; then
      cat "$result_file"
    else
      echo "(not written)"
    fi
  } >&2
}

cleanup() {
  stop_browser
  stop_server
  rm -rf "$WORK"
}
trap cleanup EXIT

yaml_pack_json="$WORK/yaml-pack.json"
core_tarball="${SPANNERPLAN_CORE_TARBALL:-}"
cli_tarball="${SPANNERPLAN_CLI_TARBALL:-}"
if [[ -n "$core_tarball" || -n "$cli_tarball" ]]; then
  if [[ -z "$core_tarball" || -z "$cli_tarball" ]]; then
    echo "SPANNERPLAN_CORE_TARBALL and SPANNERPLAN_CLI_TARBALL must be supplied together" >&2
    exit 1
  fi
  [[ -f "$core_tarball" ]] || { echo "core tarball not found: $core_tarball" >&2; exit 1; }
  [[ -f "$cli_tarball" ]] || { echo "CLI tarball not found: $cli_tarball" >&2; exit 1; }
  core_tarball="$(cd "$(dirname "$core_tarball")" && pwd)/$(basename "$core_tarball")"
  cli_tarball="$(cd "$(dirname "$cli_tarball")" && pwd)/$(basename "$cli_tarball")"
else
  core_pack_json="$WORK/core-pack.json"
  cli_pack_json="$WORK/cli-pack.json"
  (cd "$JS_ROOT" && npm_config_cache="$WORK/pack-cache" npm pack -w @spannerplan/core --pack-destination "$WORK" --json >"$core_pack_json")
  (cd "$JS_ROOT" && npm_config_cache="$WORK/pack-cache" npm pack -w @spannerplan/cli --pack-destination "$WORK" --json >"$cli_pack_json")
  core_tarball="$WORK/$(jq -r '.[0].filename' "$core_pack_json")"
  cli_tarball="$WORK/$(jq -r '.[0].filename' "$cli_pack_json")"
fi
(cd "$JS_ROOT" && npm_config_cache="$WORK/pack-cache" npm pack "$JS_ROOT/node_modules/yaml" --pack-destination "$WORK" --json >"$yaml_pack_json")
yaml_tarball="$WORK/$(jq -r '.[0].filename' "$yaml_pack_json")"

mkdir -p "$WORK/consumer"
# yaml is a public registry dependency. Supplying its local tarball keeps this
# regression check offline; the documented install itself remains the exact
# two-tarball invocation needed for the unpublished private packages.
(cd "$WORK/consumer" && npm install --offline --cache "$WORK/install-cache" --ignore-scripts "$yaml_tarball")
(cd "$WORK/consumer" && npm install --offline --cache "$WORK/install-cache" --ignore-scripts "$core_tarball" "$cli_tarball")
rendered="$WORK/rendered.txt"
"$WORK/consumer/node_modules/.bin/rendertree" -mode plan <"$REPO_ROOT/testdata/reference/dca.yaml" >"$rendered"
grep -q 'Operator' "$rendered"

browser_consumer="$WORK/browser-consumer"
mkdir -p "$browser_consumer/public"
cp "$JS_ROOT/examples/release-browser-smoke/index.html" "$browser_consumer/index.html"
cp "$JS_ROOT/examples/release-browser-smoke/main.js" "$browser_consumer/main.js"
cp "$REPO_ROOT/testdata/reference/dca.yaml" "$browser_consumer/public/dca.yaml"
# Vite is intentionally invoked from the checked-out workspace. The consumer
# itself is clean and resolves @spannerplan/core only from the packed tarball.
(cd "$browser_consumer" && npm install --offline --cache "$WORK/install-cache" --ignore-scripts "$yaml_tarball")
(cd "$browser_consumer" && npm install --offline --cache "$WORK/install-cache" --ignore-scripts "$core_tarball")
(cd "$browser_consumer" && "$JS_ROOT/node_modules/.bin/vite" build --logLevel error)

chrome_bin="${CHROME_BIN:-}"
if [[ -z "$chrome_bin" ]]; then
  for candidate in google-chrome google-chrome-stable chromium chromium-browser \
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"; do
    if command -v "$candidate" >/dev/null 2>&1; then
      chrome_bin="$(command -v "$candidate")"
      break
    elif [[ -x "$candidate" ]]; then
      chrome_bin="$candidate"
      break
    fi
  done
fi
if [[ -z "$chrome_bin" || ! -x "$chrome_bin" ]]; then
  echo "release browser smoke requires Chrome/Chromium (set CHROME_BIN)" >&2
  exit 1
fi

run_browser_smoke() {
  local mime="$1"
  local label="$2"
  local server_log="$WORK/server-${label}.log"
  local chrome_log="$WORK/chrome-${label}.log"
  local result_file="$WORK/result-${label}.json"
  local profile_dir="$WORK/chrome-profile-${label}"
  local port=""

  stop_browser
  stop_server
  mkdir "$profile_dir"
  : >"$chrome_log"
  RELEASE_SMOKE_WASM_MIME="$mime" node "$JS_ROOT/examples/release-browser-smoke/server.mjs" \
    "$browser_consumer/dist" "$result_file" >"$server_log" 2>&1 &
  server_pid=$!
  for _ in {1..50}; do
    if [[ -s "$server_log" ]]; then
      port="$(sed -n '1p' "$server_log")"
      [[ "$port" =~ ^[0-9]+$ ]] && break
    fi
    sleep 0.1
  done
  if [[ ! "$port" =~ ^[0-9]+$ ]]; then
    stop_server
    print_browser_smoke_failure "$label" "server did not report a listening port" \
      "$chrome_log" "$server_log" "$result_file"
    return 1
  fi
  "$chrome_bin" --headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage \
    "--user-data-dir=${profile_dir}" --remote-debugging-port=0 \
    "http://127.0.0.1:${port}/" >/dev/null 2>"$chrome_log" &
  browser_pid=$!
  for _ in {1..300}; do
    if [[ -s "$result_file" ]]; then
      break
    fi
    if ! kill -0 "$browser_pid" 2>/dev/null; then
      wait "$browser_pid" 2>/dev/null || true
      browser_pid=""
      stop_server
      print_browser_smoke_failure "$label" "Chrome exited before writing a page result" \
        "$chrome_log" "$server_log" "$result_file"
      return 1
    fi
    sleep 0.1
  done
  if [[ ! -s "$result_file" ]]; then
    stop_browser
    stop_server
    print_browser_smoke_failure "$label" "timed out after 30 seconds waiting for a page result" \
      "$chrome_log" "$server_log" "$result_file"
    return 1
  fi
  stop_browser
  stop_server
  if ! jq -e '
    .status == "ok"
    and .contractVersion == 1
    and (.rowCount | type == "number" and . > 0)
    and .rootNodeId == 0
    and .rootNodeText == "Distributed Union on AlbumsByAlbumTitle <Row>"
    and (.predicateLinks | type == "number" and . > 0)
  ' "$result_file" >/dev/null; then
    print_browser_smoke_failure "$label" "page result did not satisfy the release contract" \
      "$chrome_log" "$server_log" "$result_file"
    return 1
  fi
  echo "browser smoke passed (${label}, WASM MIME ${mime})"
}

# Must-have: normal application/wasm serving. Also exercise wasm-bindgen's
# generated instantiateStreaming fallback when the MIME is intentionally wrong.
run_browser_smoke "application/wasm" "native-mime"
run_browser_smoke "application/octet-stream" "fallback-mime"
