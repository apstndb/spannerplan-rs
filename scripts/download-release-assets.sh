#!/usr/bin/env bash
# Download exact GitHub Release assets through the authenticated REST API.
#
# `gh release download <tag>` intentionally does not resolve draft releases.
# Release verification runs while the release remains a draft, so it needs the
# numeric release ID and the assets API rather than the public download URL.
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage: download-release-assets.sh --repo OWNER/REPO --release-id ID --dir DIR ASSET [ASSET ...]
EOF
  exit 2
}

REPO=""
RELEASE_ID=""
DESTINATION=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      [[ $# -ge 2 ]] || usage
      REPO="$2"
      shift 2
      ;;
    --release-id)
      [[ $# -ge 2 ]] || usage
      RELEASE_ID="$2"
      shift 2
      ;;
    --dir)
      [[ $# -ge 2 ]] || usage
      DESTINATION="$2"
      shift 2
      ;;
    --help|-h)
      usage
      ;;
    --*)
      echo "error: unknown option: $1" >&2
      usage
      ;;
    *)
      break
      ;;
  esac
done

[[ -n "$REPO" && -n "$RELEASE_ID" && -n "$DESTINATION" && $# -gt 0 ]] || usage
[[ "$REPO" == */* && "$REPO" != */*/* ]] || {
  echo "error: --repo must be OWNER/REPO" >&2
  exit 2
}
[[ "$RELEASE_ID" =~ ^[0-9]+$ ]] || {
  echo "error: --release-id must be numeric" >&2
  exit 2
}

for asset_name in "$@"; do
  [[ "$asset_name" != */* && "$asset_name" != . && "$asset_name" != .. ]] || {
    echo "error: asset name must be a basename: $asset_name" >&2
    exit 2
  }
done

if [[ "$(printf '%s\n' "$@" | LC_ALL=C sort | uniq -d)" != "" ]]; then
  echo "error: asset names must be unique" >&2
  exit 2
fi

mkdir -p "$DESTINATION"
ASSET_LIST="$(mktemp)"
trap 'rm -f "$ASSET_LIST"' EXIT

# `--paginate` keeps this correct if a release eventually grows beyond one
# assets page. The releases API remains available to an authenticated workflow
# token for drafts owned by the repository.
gh api --paginate "repos/${REPO}/releases/${RELEASE_ID}/assets?per_page=100" >"$ASSET_LIST"

expected_asset_names="$(printf '%s\n' "$@" | LC_ALL=C sort)"
actual_asset_names="$(jq -ers '.[][] | .name' "$ASSET_LIST" | LC_ALL=C sort)"
if [[ "$actual_asset_names" != "$expected_asset_names" ]]; then
  {
    echo "error: release asset set differs from the expected set"
    echo "expected:"
    printf '%s\n' "$expected_asset_names"
    echo "actual:"
    printf '%s\n' "$actual_asset_names"
  } >&2
  exit 1
fi

for asset_name in "$@"; do
  asset_id="$(jq -ers --arg name "$asset_name" '
    [ .[][] | select(.name == $name) ]
    | if length == 1 then .[0].id
      elif length == 0 then error("missing release asset: " + $name)
      else error("duplicate release asset: " + $name)
      end
  ' "$ASSET_LIST")"
  target="$DESTINATION/$asset_name"
  temporary="$target.partial"
  rm -f "$temporary"

  # The binary endpoint is authenticated; browser_download_url is unsuitable
  # here because a draft release is intentionally not public yet.
  gh api -H 'Accept: application/octet-stream' \
    "repos/${REPO}/releases/assets/${asset_id}" >"$temporary"
  if [[ ! -s "$temporary" ]]; then
    echo "error: downloaded asset is empty: $asset_name" >&2
    exit 1
  fi
  mv "$temporary" "$target"
done
