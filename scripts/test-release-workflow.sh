#!/usr/bin/env bash
# Keep tag lookup data separate from the jq program in the release workflow.
# The grep expressions below intentionally match literal shell and jq source.
# shellcheck disable=SC2016
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORKFLOW="$ROOT/.github/workflows/release.yml"

if grep -F 'select(.tag_name == "${TAG}")' "$WORKFLOW" >/dev/null; then
  echo "error: release workflow interpolates TAG into jq source" >&2
  exit 1
fi

if grep -F 'sed -i "s/TAG/${TAG}/g"' "$WORKFLOW" >/dev/null; then
  echo "error: release workflow interpolates TAG into a sed program" >&2
  exit 1
fi

if ! grep -F -- '--arg tag "$TAG"' "$WORKFLOW" | grep -F 'gsub("TAG"; $tag)' >/dev/null; then
  echo "error: release notes do not substitute TAG as jq data" >&2
  exit 1
fi

require_safe_tag_lookup() {
  local variable="$1"
  if ! grep -F -A2 "${variable}=\"\$(gh api" "$WORKFLOW" | \
    grep -F -- '--arg tag "$TAG"' | \
    grep -F 'select(.tag_name == $tag)' >/dev/null; then
    echo "error: $variable does not select the tag through jq data" >&2
    exit 1
  fi
}

require_safe_tag_lookup "EXISTING_RELEASE_IDS"
require_safe_tag_lookup "RELEASE_ID"

echo "release workflow tag lookup safety check passed"
