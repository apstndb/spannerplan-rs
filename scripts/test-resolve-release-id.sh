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

[[ "$1" == "api" && "$2" == "--paginate" ]] || {
  echo "unexpected gh arguments: $*" >&2
  exit 1
}
[[ "$#" -eq 3 ]] || {
  echo "unexpected gh argument count: $# ($*)" >&2
  exit 1
}
[[ "$3" == "repos/apstndb/spannerplan-rs/releases?per_page=100" ]] || {
  echo "unexpected release endpoint: $3" >&2
  exit 1
}

attempt=0
if [[ -f "$MOCK_STATE" ]]; then
  attempt="$(<"$MOCK_STATE")"
fi
attempt=$((attempt + 1))
printf '%s\n' "$attempt" >"$MOCK_STATE"

case "$MOCK_MODE" in
  empty-then-sole-draft)
    if ((attempt >= 3)); then
      printf '%s\n' '[{"id":355929622,"tag_name":"v0.1.0-alpha.3","draft":true}]'
    else
      printf '%s\n' '[]'
    fi
    ;;
  multi-page)
    printf '%s\n' '[{"id":355929620,"tag_name":"v0.1.0-alpha.2","draft":false}]'
    printf '%s\n' '[{"id":355929622,"tag_name":"v0.1.0-alpha.3","draft":true}]'
    ;;
  always-empty)
    printf '%s\n' '[]'
    ;;
  duplicate-drafts)
    printf '%s\n' '[{"id":355929622,"tag_name":"v0.1.0-alpha.3","draft":true},{"id":355929623,"tag_name":"v0.1.0-alpha.3","draft":true}]'
    ;;
  published-and-draft-pages)
    printf '%s\n' '[{"id":355929621,"tag_name":"v0.1.0-alpha.3","draft":false}]'
    printf '%s\n' '[{"id":355929622,"tag_name":"v0.1.0-alpha.3","draft":true}]'
    ;;
  sole-published)
    printf '%s\n' '[{"id":355929621,"tag_name":"v0.1.0-alpha.3","draft":false}]'
    ;;
  string-draft)
    printf '%s\n' '[{"id":355929622,"tag_name":"v0.1.0-alpha.3","draft":"true"}]'
    ;;
  object-page)
    printf '%s\n' '{"release":{"id":355929622,"tag_name":"v0.1.0-alpha.3","draft":true}}'
    ;;
  empty-body)
    ;;
  mixed-malformed-and-valid)
    printf '%s\n' '[null,{}, {"id":355929622,"tag_name":"v0.1.0-alpha.3","draft":true}]'
    ;;
  sole-malformed-entry)
    printf '%s\n' '[{}]'
    ;;
  fatal)
    echo 'gh: Resource not accessible by integration (HTTP 403)' >&2
    exit 1
    ;;
  malformed)
    printf '%s\n' '<html>not JSON</html>'
    ;;
  nonnumeric)
    printf '%s\n' '[{"id":"not-a-number","tag_name":"v0.1.0-alpha.3","draft":true}]'
    ;;
  numeric-string)
    printf '%s\n' '[{"id":"355929622","tag_name":"v0.1.0-alpha.3","draft":true}]'
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

state="$WORK/empty-then-sole-draft.state"
output="$(run_resolver 4 empty-then-sole-draft "$state")"
test "$output" = '355929622'
test "$(<"$state")" = '3'

state="$WORK/multi-page.state"
output="$(run_resolver 3 multi-page "$state")"
test "$output" = '355929622'
test "$(<"$state")" = '1'

state="$WORK/always-empty.state"
if run_resolver 3 always-empty "$state" >"$WORK/always-empty.out" 2>"$WORK/always-empty.err"; then
  echo "error: always-empty lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '3'
grep -F "error: draft release ID for tag 'v0.1.0-alpha.3' was not visible after 3 attempts" "$WORK/always-empty.err" >/dev/null

state="$WORK/duplicate-drafts.state"
if run_resolver 3 duplicate-drafts "$state" >"$WORK/duplicate-drafts.out" 2>"$WORK/duplicate-drafts.err"; then
  echo "error: duplicate lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '1'
grep -F "error: release lookup for tag 'v0.1.0-alpha.3' returned multiple release records" "$WORK/duplicate-drafts.err" >/dev/null

state="$WORK/published-and-draft-pages.state"
if run_resolver 3 published-and-draft-pages "$state" >"$WORK/published-and-draft-pages.out" 2>"$WORK/published-and-draft-pages.err"; then
  echo "error: published-and-draft lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '1'
grep -F "error: release lookup for tag 'v0.1.0-alpha.3' returned multiple release records" "$WORK/published-and-draft-pages.err" >/dev/null

state="$WORK/sole-published.state"
if run_resolver 3 sole-published "$state" >"$WORK/sole-published.out" 2>"$WORK/sole-published.err"; then
  echo "error: sole-published lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '1'
grep -F "error: the sole release for tag 'v0.1.0-alpha.3' is not a draft" "$WORK/sole-published.err" >/dev/null

state="$WORK/string-draft.state"
if run_resolver 3 string-draft "$state" >"$WORK/string-draft.out" 2>"$WORK/string-draft.err"; then
  echo "error: string-draft lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '1'
grep -F "error: the sole release for tag 'v0.1.0-alpha.3' is not a draft" "$WORK/string-draft.err" >/dev/null

for mode in object-page empty-body mixed-malformed-and-valid sole-malformed-entry; do
  state="$WORK/$mode.state"
  if run_resolver 3 "$mode" "$state" >"$WORK/$mode.out" 2>"$WORK/$mode.err"; then
    echo "error: $mode lookup unexpectedly succeeded" >&2
    exit 1
  fi
  test "$(<"$state")" = '1'
  grep -F "error: release list for tag 'v0.1.0-alpha.3' was not valid GitHub API JSON" "$WORK/$mode.err" >/dev/null
done

state="$WORK/fatal.state"
if run_resolver 3 fatal "$state" >"$WORK/fatal.out" 2>"$WORK/fatal.err"; then
  echo "error: fatal lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '1'
grep -F "error: release lookup for tag 'v0.1.0-alpha.3' failed:" "$WORK/fatal.err" >/dev/null
grep -F 'gh: Resource not accessible by integration (HTTP 403)' "$WORK/fatal.err" >/dev/null

state="$WORK/malformed.state"
if run_resolver 3 malformed "$state" >"$WORK/malformed.out" 2>"$WORK/malformed.err"; then
  echo "error: malformed lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '1'
grep -F "error: release list for tag 'v0.1.0-alpha.3' was not valid GitHub API JSON" "$WORK/malformed.err" >/dev/null

state="$WORK/nonnumeric.state"
if run_resolver 3 nonnumeric "$state" >"$WORK/nonnumeric.out" 2>"$WORK/nonnumeric.err"; then
  echo "error: nonnumeric lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '1'
grep -F "error: release lookup for tag 'v0.1.0-alpha.3' returned a non-numeric release ID" "$WORK/nonnumeric.err" >/dev/null

state="$WORK/numeric-string.state"
if run_resolver 3 numeric-string "$state" >"$WORK/numeric-string.out" 2>"$WORK/numeric-string.err"; then
  echo "error: numeric-string lookup unexpectedly succeeded" >&2
  exit 1
fi
test "$(<"$state")" = '1'
grep -F "error: release lookup for tag 'v0.1.0-alpha.3' returned a non-numeric release ID" "$WORK/numeric-string.err" >/dev/null

echo "release ID resolver smoke tests passed"
