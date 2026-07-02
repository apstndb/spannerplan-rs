#!/usr/bin/env bash
# Smoke-test consumer installs for a tagged release (run locally after push).
set -euo pipefail

TAG="${1:-v0.1.0-alpha.1}"
REPO="${SPANNERPLAN_REPO:-https://github.com/apstndb/spannerplan-rs}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "==> Rust git dependency ($TAG)"
mkdir -p "$WORK/rust-consumer/src"
cat > "$WORK/rust-consumer/Cargo.toml" <<EOF
[package]
name = "consumer"
version = "0.0.0"
edition = "2021"

[dependencies]
spannerplan = { git = "$REPO", tag = "$TAG" }
EOF
cat > "$WORK/rust-consumer/src/main.rs" <<'EOF'
fn main() {
    let yaml = include_str!("plan.yaml");
    let nodes = spannerplan::extract::extract_plan_nodes(yaml.as_bytes()).unwrap();
    assert!(!nodes.is_empty());
    println!("ok: {} plan nodes", nodes.len());
}
EOF
cp "$ROOT/testdata/reference/dca.yaml" "$WORK/rust-consumer/src/plan.yaml"
cargo run --manifest-path "$WORK/rust-consumer/Cargo.toml"

echo "==> FFI library (local build)"
(cd "$ROOT" && cargo build -p spannerplan-ffi --release)
export SPANNERPLAN_FFI_LIB="$ROOT/target/release/libspannerplan_ffi.dylib"
if [[ ! -f "$SPANNERPLAN_FFI_LIB" ]]; then
  export SPANNERPLAN_FFI_LIB="$ROOT/target/release/libspannerplan_ffi.so"
fi

echo "==> Python git install ($TAG)"
pip install -q "spannerplan @ git+${REPO}@${TAG}#subdirectory=bindings/python"
python3 -c "
from spannerplan import render_tree_table_json
from pathlib import Path
p = Path('$ROOT/testdata/reference/dca.yaml').read_text()
out = render_tree_table_json(p, 'PLAN', 'CURRENT')
assert out and 'Operator' in out
print('ok: python render', len(out), 'bytes')
"

echo "==> npm tarball from workspace build"
(cd "$ROOT/js" && npm run build && npm pack -w @spannerplan/core)
TARBALL="$(ls -1 "$ROOT/js"/spannerplan-core-*.tgz | head -1)"
mkdir -p "$WORK/npm-consumer"
cat > "$WORK/npm-consumer/package.json" <<EOF
{ "type": "module", "dependencies": { "@spannerplan/core": "file:$TARBALL" } }
EOF
cat > "$WORK/npm-consumer/test.mjs" <<'EOF'
import { readFileSync } from "node:fs";
import { renderTreeTable } from "@spannerplan/core";
const yaml = readFileSync(process.argv[2], "utf8");
const result = renderTreeTable(yaml, "PLAN", "CURRENT");
if ("error" in result) throw new Error(result.error);
console.log("ok: js render", result.output.length, "bytes");
EOF
(cd "$WORK/npm-consumer" && npm install -q && node test.mjs "$ROOT/testdata/reference/dca.yaml")

echo "All consumer smoke tests passed for $TAG"
