#!/usr/bin/env bash
# Keep tag lookup data separate from jq source in the release workflow, and
# ensure the post-create lookup delegates to the retrying exact-resource helper.
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

if grep -F 'RELEASE_ID="$(gh api' "$WORKFLOW" >/dev/null; then
  echo "error: release workflow still resolves the post-create release ID through gh api inline" >&2
  exit 1
fi

RESOLVER_INVOCATION="$(grep -F -A2 'scripts/resolve-release-id.sh' "$WORKFLOW")"
if ! grep -F -- '--repo "$GITHUB_REPOSITORY"' <<<"$RESOLVER_INVOCATION" >/dev/null || \
  ! grep -F -- '--tag "$TAG"' <<<"$RESOLVER_INVOCATION" >/dev/null; then
  echo "error: release workflow does not use the exact-resource release-ID resolver" >&2
  exit 1
fi

bash "$ROOT/scripts/test-resolve-release-id.sh"

echo "release workflow tag lookup safety check passed"
