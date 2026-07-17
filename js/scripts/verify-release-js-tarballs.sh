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
stop_server() {
  local pid="$server_pid"
  server_pid=""
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
  local dom="$5"
  local dom_status=""

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
    echo "--- browser DOM (${label}) ---"
    if [[ -f "$dom" ]]; then
      dom_status="$(grep -Eom1 'data-status="[^"]*"' "$dom" || true)"
      if [[ -n "$dom_status" ]]; then
        echo "DOM status: ${dom_status}"
      else
        echo "DOM status: no data-status attribute"
      fi
      cat "$dom"
    else
      echo "(not written)"
    fi
  } >&2
}

cleanup() {
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
  local dom="$WORK/dom-${label}.html"
  local profile_dir="$WORK/chrome-profile-${label}"
  local port=""
  local chrome_status=0

  stop_server
  mkdir "$profile_dir"
  : >"$chrome_log"
  RELEASE_SMOKE_WASM_MIME="$mime" node "$JS_ROOT/examples/release-browser-smoke/server.mjs" \
    "$browser_consumer/dist" >"$server_log" 2>&1 &
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
      "$chrome_log" "$server_log" "$dom"
    return 1
  fi
  if "$chrome_bin" --headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage \
    "--user-data-dir=${profile_dir}" --dump-dom --virtual-time-budget=10000 \
    "http://127.0.0.1:${port}/" >"$dom" 2>"$chrome_log"; then
    chrome_status=0
  else
    chrome_status=$?
  fi
  stop_server
  if [[ "$chrome_status" -ne 0 ]]; then
    print_browser_smoke_failure "$label" "Chrome exited with status ${chrome_status}" \
      "$chrome_log" "$server_log" "$dom"
    return 1
  fi
  if ! grep -Eq 'data-status="ok"' "$dom"; then
    print_browser_smoke_failure "$label" "DOM assertion failed: data-status=ok" \
      "$chrome_log" "$server_log" "$dom"
    return 1
  fi
  if ! grep -Eq '"contractVersion":1' "$dom"; then
    print_browser_smoke_failure "$label" "DOM assertion failed: contractVersion=1" \
      "$chrome_log" "$server_log" "$dom"
    return 1
  fi
  if ! grep -Eq '"rowCount":[1-9][0-9]*' "$dom"; then
    print_browser_smoke_failure "$label" "DOM assertion failed: nonzero rowCount" \
      "$chrome_log" "$server_log" "$dom"
    return 1
  fi
  if ! grep -Eq '"rootNodeId":0' "$dom"; then
    print_browser_smoke_failure "$label" "DOM assertion failed: rootNodeId=0" \
      "$chrome_log" "$server_log" "$dom"
    return 1
  fi
  if ! grep -Fq '"rootNodeText":"Distributed Union on AlbumsByAlbumTitle &lt;Row&gt;"' "$dom"; then
    print_browser_smoke_failure "$label" "DOM assertion failed: root node text" \
      "$chrome_log" "$server_log" "$dom"
    return 1
  fi
  if ! grep -Eq '"predicateLinks":[1-9][0-9]*' "$dom"; then
    print_browser_smoke_failure "$label" "DOM assertion failed: nonzero predicateLinks" \
      "$chrome_log" "$server_log" "$dom"
    return 1
  fi
  echo "browser smoke passed (${label}, WASM MIME ${mime})"
}

# Must-have: normal application/wasm serving. Also exercise wasm-bindgen's
# generated instantiateStreaming fallback when the MIME is intentionally wrong.
run_browser_smoke "application/wasm" "native-mime"
run_browser_smoke "application/octet-stream" "fallback-mime"
