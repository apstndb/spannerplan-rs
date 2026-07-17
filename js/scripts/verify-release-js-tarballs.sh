#!/usr/bin/env bash
# Verify that the release core and CLI packages can be installed and run together.
set -euo pipefail

JS_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$JS_ROOT/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

core_pack_json="$WORK/core-pack.json"
cli_pack_json="$WORK/cli-pack.json"
yaml_pack_json="$WORK/yaml-pack.json"
(cd "$JS_ROOT" && npm_config_cache="$WORK/pack-cache" npm pack -w @spannerplan/core --pack-destination "$WORK" --json >"$core_pack_json")
(cd "$JS_ROOT" && npm_config_cache="$WORK/pack-cache" npm pack -w @spannerplan/cli --pack-destination "$WORK" --json >"$cli_pack_json")
(cd "$JS_ROOT" && npm_config_cache="$WORK/pack-cache" npm pack "$JS_ROOT/node_modules/yaml" --pack-destination "$WORK" --json >"$yaml_pack_json")
core_tarball="$WORK/$(jq -r '.[0].filename' "$core_pack_json")"
cli_tarball="$WORK/$(jq -r '.[0].filename' "$cli_pack_json")"
yaml_tarball="$WORK/$(jq -r '.[0].filename' "$yaml_pack_json")"

mkdir -p "$WORK/consumer"
# yaml is a public registry dependency. Supplying its local tarball keeps this
# regression check offline; the documented install itself remains the exact
# two-tarball invocation needed for the unpublished private packages.
(cd "$WORK/consumer" && npm install --offline --cache "$WORK/install-cache" --ignore-scripts "$yaml_tarball")
(cd "$WORK/consumer" && npm install --offline --cache "$WORK/install-cache" --ignore-scripts "$core_tarball" "$cli_tarball")
rendered="$WORK/rendered.txt"
"$WORK/consumer/node_modules/.bin/rendertree" -mode plan <"$REPO_ROOT/testdata/reference/dca.yaml" >"$rendered"
grep -q 'Operator' "$rendered"
