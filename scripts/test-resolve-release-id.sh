#!/usr/bin/env bash
# Exercise release-ID eventual-consistency handling without contacting GitHub.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$(mktemp -d)"
cleanup() {
  rm -rf "$WORK"
}
trap cleanup EXIT

MOCK_BIN="$WORK/bin"
mkdir -p "$MOCK_BIN"
cat >"$MOCK_BIN/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

[[ "$1" == "api" && "$2" == "--jq" && "$3" == ".id" ]] || {
  echo "unexpected gh arguments: $*" >&2
  exit 1
}
[[ "$4" == "repos/apstndb/spannerplan-rs/releases/tags/v0.1.0-alpha.3" ]] || {
  echo "unexpected release endpoint: $4" >&2
  exit 1
}

attempt=0
if [[ -f "$MOCK_STATE" ]]; then
  attempt="$(<"$MOCK_STATE")"
fi
attempt=$((attempt + 1))
printf '%s\n' "$attempt" >"$MOCK_STATE"

case "$MOCK_MODE" in
  not-found-then-success)
    if ((attempt < 3)); then
      echo 'gh: Not Found (HTTP 404)' >&2
      exit 1
    fi
    printf '%s\n' '355929622'
    ;;
  empty-then-success)
    if ((attempt >= 3)); then
      printf '%s\n' '355929622'
    fi
    ;;
  always-empty)
    ;;
  duplicate)
    printf '%s\n' '355929622' '355929623'
    ;;
  fatal)
    echo 'gh: Resource not accessible by integration (HTTP 403)' >&2
    exit 1
    ;;
  *)
    echo "unknown mock mode: $MOCK_MODE" >&2
    exit 1
    ;;
esac
EOF
chmod +x "$MOCK_BIN/gh"

run_resolver() {
  PATH="$MOCK_BIN:$PATH" \
    RELEASE_ID_SKIP_SLEEP=1 \
    RELEASE_ID_MAX_ATTEMPTS="$1" \
    MOCK_MODE="$2" \
    MOCK_STATE="$3" \
    bash "$ROOT/scripts/resolve-release-id.sh" \
      --repo apstndb/spannerplan-rs \
      --tag v0.1.0-alpha.3
}

state="$WORK/empty-then-success.state"
output="$(run_resolver 4 empty-then-success "$state")"
test "$output" = '355929622'
test "$(<"$state")" = '3'

state="$WORK/not-found-then-success.state"
output="$(run_resolver 4 not-found-then-success "$state")"
test "$output" = '355929622'
test "$(<"$state")" = '3'

state="$WORK/always-empty.state"
if run_resolver 3 always-empty "$state" >"$WORK/always-empty.out" 2>"$WORK/always-empty.err"; then
  echo "error: always-empty lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '3'
grep -F "error: release ID for tag 'v0.1.0-alpha.3' was not visible after 3 attempts" "$WORK/always-empty.err" >/dev/null

state="$WORK/duplicate.state"
if run_resolver 3 duplicate "$state" >"$WORK/duplicate.out" 2>"$WORK/duplicate.err"; then
  echo "error: duplicate lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '1'
grep -F "error: release lookup for tag 'v0.1.0-alpha.3' returned multiple release IDs" "$WORK/duplicate.err" >/dev/null

state="$WORK/fatal.state"
if run_resolver 3 fatal "$state" >"$WORK/fatal.out" 2>"$WORK/fatal.err"; then
  echo "error: fatal lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '1'
grep -F "error: release lookup for tag 'v0.1.0-alpha.3' failed:" "$WORK/fatal.err" >/dev/null
grep -F 'gh: Resource not accessible by integration (HTTP 403)' "$WORK/fatal.err" >/dev/null

echo "release ID resolver smoke tests passed"
