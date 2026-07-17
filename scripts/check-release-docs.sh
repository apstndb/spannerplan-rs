#!/usr/bin/env bash
# Keep release download commands portable outside a repository checkout.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAILED=0

while IFS=: read -r doc_file line_number command; do
  if [[ "$command" != *"--repo "* ]]; then
    echo "error: $doc_file:$line_number: gh release download must specify --repo" >&2
    FAILED=1
  fi
done < <(git -C "$ROOT" grep -n -F 'gh release download' -- '*.md' '.github/workflows/*.yml' || true)

UNSUPPORTED_PATTERN='github:apstndb/spannerplan-rs#[^[:space:]"`]*(&|%26)path:'
if unsupported="$(git -C "$ROOT" grep -n -E "$UNSUPPORTED_PATTERN" -- '*.md')"; then
  echo "error: unsupported npm GitHub subdirectory dependency syntax:" >&2
  printf '%s\n' "$unsupported" >&2
  FAILED=1
fi

if ((FAILED != 0)); then
  exit 1
fi

echo "Release documentation checks passed."
