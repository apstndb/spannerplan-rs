#!/usr/bin/env bash
# Smoke-test release checksum verification and optionally assert the selected tool.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXPECTED_TOOL="${1:-}"
WORK="$(mktemp -d)"
cleanup() {
  rm -rf "$WORK"
}
trap cleanup EXIT

printf 'spannerplan checksum fixture\n' >"$WORK/asset.txt"
printf '%s  %s\n' \
  '6e5541de5313816a69ccc4800cc237be420cfa84da71b758d21ef9062c935c7b' \
  'asset.txt' >"$WORK/SHA256SUMS.txt"

OUTPUT="$(bash "$ROOT/scripts/verify-release-checksums.sh" "$WORK/SHA256SUMS.txt")"
printf '%s\n' "$OUTPUT"

if [[ -n "$EXPECTED_TOOL" && "$OUTPUT" != *"using $EXPECTED_TOOL"* ]]; then
  echo "error: expected checksum verifier to use $EXPECTED_TOOL" >&2
  exit 1
fi
