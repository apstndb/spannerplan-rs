#!/usr/bin/env bash
# Keep tag lookup data separate from the jq program in the release workflow.
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

tag_argument_count="$(grep -F -- '--arg tag "$TAG"' "$WORKFLOW" | wc -l | tr -d ' ')"
if [[ "$tag_argument_count" != "3" ]]; then
  echo "error: expected three jq --arg tag uses, found $tag_argument_count" >&2
  exit 1
fi

tag_filter_count="$(grep -F 'select(.tag_name == $tag)' "$WORKFLOW" | wc -l | tr -d ' ')"
if [[ "$tag_filter_count" != "2" ]]; then
  echo "error: expected two jq tag filters, found $tag_filter_count" >&2
  exit 1
fi

echo "release workflow tag lookup safety check passed"
