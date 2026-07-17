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
  local expected="${3:-$EXPECTED_VERSION}"
  if [[ "$actual" != "$expected" ]]; then
    echo "error: $label version '$actual' does not match tag '$TAG'" >&2
    exit 1
  fi
  echo "ok: $label = $actual"
}

check_version "Cargo workspace" "$CARGO_VERSION"

FOUND_CORE=0
FOUND_CLI=0
for manifest in "$ROOT"/js/packages/*/package.json; do
  [[ -f "$manifest" ]] || continue
  IFS=$'\t' read -r PACKAGE_NAME PACKAGE_VERSION < <(
    node -p 'const p = require(process.argv[1]); [p.name ?? "", p.version ?? ""].join("\t")' "$manifest"
  )
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

PYTHON_VERSION="$(awk -F '"' '/^version[[:space:]]*=/ { print $2; exit }' "$ROOT/bindings/python/pyproject.toml")"
PHP_VERSION="$(node -p 'require(process.argv[1]).version' "$ROOT/bindings/php/composer.json")"
JAVA_VERSION="$(awk -F '[<>]' '/^[[:space:]]*<version>/ { print $3; exit }' "$ROOT/bindings/java/pom.xml")"
RUBY_VERSION="$(awk -F "'" '/^[[:space:]]*s\.version[[:space:]]*=/ { print $2; exit }' "$ROOT/bindings/ruby/spannerplan.gemspec")"

check_version "Python binding" "$PYTHON_VERSION"
check_version "PHP binding" "$PHP_VERSION"
check_version "Java binding" "$JAVA_VERSION"
check_version "Ruby binding" "$RUBY_VERSION" "${EXPECTED_VERSION//-/.}"

echo "Release tag $TAG matches all release package versions."
