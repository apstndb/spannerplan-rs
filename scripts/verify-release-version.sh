#!/usr/bin/env bash
# Verify that a release tag matches every package version used in release assets.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TAG="${1:-${GITHUB_REF_NAME:-}}"

if [[ ! "$TAG" =~ ^v(.+)$ ]]; then
  echo "error: expected a release tag in the form v<version>, got '${TAG:-<empty>}'" >&2
  exit 1
fi
EXPECTED_VERSION="${BASH_REMATCH[1]}"

CARGO_VERSION="$({
  awk '
    /^\[workspace\.package\][[:space:]]*$/ { in_workspace_package = 1; next }
    /^\[/ { in_workspace_package = 0 }
    in_workspace_package && /^[[:space:]]*version[[:space:]]*=/ {
      line = $0
      sub(/^[^=]*=[[:space:]]*"/, "", line)
      sub(/"[[:space:]]*$/, "", line)
      print line
      exit
    }
  ' "$ROOT/Cargo.toml"
} || true)"

if [[ -z "$CARGO_VERSION" ]]; then
  echo "error: could not read [workspace.package].version from Cargo.toml" >&2
  exit 1
fi

check_version() {
  local label="$1"
  local actual="$2"
  if [[ "$actual" != "$EXPECTED_VERSION" ]]; then
    echo "error: $label version '$actual' does not match tag '$TAG'" >&2
    return 1
  fi
  echo "ok: $label = $actual"
}

check_version "Cargo workspace" "$CARGO_VERSION"

FOUND_CORE=0
FOUND_CLI=0
for manifest in "$ROOT"/js/packages/*/package.json; do
  [[ -f "$manifest" ]] || continue
  PACKAGE_NAME="$(node -e 'process.stdout.write(require(process.argv[1]).name || "")' "$manifest")"
  case "$PACKAGE_NAME" in
    @spannerplan/core)
      FOUND_CORE=1
      ;;
    @spannerplan/cli)
      FOUND_CLI=1
      ;;
    *)
      continue
      ;;
  esac
  PACKAGE_VERSION="$(node -e 'process.stdout.write(require(process.argv[1]).version || "")' "$manifest")"
  check_version "$PACKAGE_NAME" "$PACKAGE_VERSION"
done

if ((FOUND_CORE == 0)); then
  echo "error: could not find the @spannerplan/core package manifest under js/packages" >&2
  exit 1
fi
if ((FOUND_CLI == 0)); then
  echo "error: could not find the @spannerplan/cli package manifest under js/packages" >&2
  exit 1
fi

echo "Release tag $TAG matches all release package versions."
