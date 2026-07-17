#!/usr/bin/env bash
# Resolve a just-created draft release through its exact REST resource.
#
# GitHub's paginated releases list can briefly omit a newly created draft. The
# tag endpoint is the authoritative single-resource lookup; retry its empty or
# not-yet-visible response without ever creating, deleting, or editing a
# release.
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage: resolve-release-id.sh --repo OWNER/REPO --tag TAG
EOF
  exit 2
}

REPO=""
TAG=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      [[ $# -ge 2 ]] || usage
      REPO="$2"
      shift 2
      ;;
    --tag)
      [[ $# -ge 2 ]] || usage
      TAG="$2"
      shift 2
      ;;
    --help|-h)
      usage
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage
      ;;
  esac
done

[[ "$REPO" == */* && "$REPO" != */*/* ]] || {
  echo "error: --repo must be OWNER/REPO" >&2
  exit 2
}
[[ -n "$TAG" ]] || {
  echo "error: --tag must not be empty" >&2
  exit 2
}

MAX_ATTEMPTS="${RELEASE_ID_MAX_ATTEMPTS:-6}"
[[ "$MAX_ATTEMPTS" =~ ^[1-9][0-9]*$ ]] || {
  echo "error: RELEASE_ID_MAX_ATTEMPTS must be a positive integer" >&2
  exit 2
}

# The tag is data, not shell or jq source. It is URI-encoded before becoming
# the final path component, so unusual valid tag characters cannot alter the
# API route.
ENCODED_TAG="$(jq -rn --arg tag "$TAG" '$tag | @uri')"
ENDPOINT="repos/${REPO}/releases/tags/${ENCODED_TAG}"
DELAY_SECONDS=1
ERROR_FILE="$(mktemp)"
trap 'rm -f "$ERROR_FILE"' EXIT

for ((attempt = 1; attempt <= MAX_ATTEMPTS; attempt++)); do
  RESPONSE=""
  : >"$ERROR_FILE"
  if RESPONSE="$(gh api --jq '.id' "$ENDPOINT" 2>"$ERROR_FILE")"; then
    RELEASE_IDS="$(printf '%s\n' "$RESPONSE" | sed '/^$/d')"
    RELEASE_ID_COUNT="$(printf '%s\n' "$RELEASE_IDS" | awk 'NF { count++ } END { print count + 0 }')"
    case "$RELEASE_ID_COUNT" in
      1)
        if [[ "$RELEASE_IDS" =~ ^[0-9]+$ ]]; then
          printf '%s\n' "$RELEASE_IDS"
          exit 0
        fi
        echo "error: release lookup for tag '$TAG' returned a non-numeric release ID" >&2
        exit 1
        ;;
      0)
        # A successful empty response is the observed eventual-consistency
        # failure mode. Treat it exactly like a not-yet-visible draft.
        ;;
      *)
        echo "error: release lookup for tag '$TAG' returned multiple release IDs" >&2
        exit 1
        ;;
    esac
  elif ! grep -Eq '\(HTTP 404\)[[:space:]]*$' "$ERROR_FILE"; then
    echo "error: release lookup for tag '$TAG' failed:" >&2
    sed 's/^/  /' "$ERROR_FILE" >&2
    exit 1
  fi

  if ((attempt == MAX_ATTEMPTS)); then
    break
  fi
  if [[ "${RELEASE_ID_SKIP_SLEEP:-0}" != "1" ]]; then
    sleep "$DELAY_SECONDS"
  fi
  DELAY_SECONDS=$((DELAY_SECONDS * 2))
done

echo "error: release ID for tag '$TAG' was not visible after $MAX_ATTEMPTS attempts; verify the existing draft before retrying the workflow" >&2
exit 1
