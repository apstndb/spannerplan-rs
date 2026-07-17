#!/usr/bin/env bash
# Verify a release checksum manifest with GNU sha256sum or stock macOS shasum.
set -euo pipefail

MANIFEST="${1:-SHA256SUMS.txt}"
if [[ ! -f "$MANIFEST" ]]; then
  echo "error: checksum manifest not found: $MANIFEST" >&2
  exit 1
fi

MANIFEST_DIR="$(cd "$(dirname "$MANIFEST")" && pwd)"
MANIFEST_NAME="$(basename "$MANIFEST")"

if command -v sha256sum >/dev/null 2>&1; then
  echo "Verifying $MANIFEST_NAME using sha256sum"
  (cd "$MANIFEST_DIR" && sha256sum -c "$MANIFEST_NAME")
elif command -v shasum >/dev/null 2>&1; then
  echo "Verifying $MANIFEST_NAME using shasum"
  (cd "$MANIFEST_DIR" && shasum -a 256 -c "$MANIFEST_NAME")
else
  echo "error: SHA-256 verification requires sha256sum or shasum" >&2
  exit 1
fi
