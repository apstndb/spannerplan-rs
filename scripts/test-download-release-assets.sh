#!/usr/bin/env bash
# Exercise draft-release REST asset lookup/download without contacting GitHub.
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

for argument in "$@"; do
  case "$argument" in
    *'/assets?per_page=100')
      printf '%s\n' '[{"id":101,"name":"SHA256SUMS.txt"},{"id":102,"name":"spannerplan-core-0.1.0-alpha.2.tgz"}]'
      exit 0
      ;;
    */assets/101)
      printf 'checksum fixture\n'
      exit 0
      ;;
    */assets/102)
      printf 'tarball fixture\n'
      exit 0
      ;;
  esac
done

echo "unexpected gh invocation: $*" >&2
exit 1
EOF
chmod +x "$MOCK_BIN/gh"

PATH="$MOCK_BIN:$PATH" bash "$ROOT/scripts/download-release-assets.sh" \
  --repo apstndb/spannerplan-rs \
  --release-id 355624153 \
  --dir "$WORK/assets" \
  SHA256SUMS.txt spannerplan-core-0.1.0-alpha.2.tgz

test "$(cat "$WORK/assets/SHA256SUMS.txt")" = "checksum fixture"
test "$(cat "$WORK/assets/spannerplan-core-0.1.0-alpha.2.tgz")" = "tarball fixture"

if PATH="$MOCK_BIN:$PATH" bash "$ROOT/scripts/download-release-assets.sh" \
  --repo apstndb/spannerplan-rs \
  --release-id 355624153 \
  --dir "$WORK/missing" \
  missing-asset.tgz >/dev/null 2>&1; then
  echo "error: missing release asset unexpectedly succeeded" >&2
  exit 1
fi

echo "draft release asset downloader smoke test passed"
