#!/usr/bin/env bash
# Verify only assets published for a tagged release; never fall back to a
# workspace-built native library or JavaScript package.
set -euo pipefail

TAG="${1:-v0.1.0-alpha.2}"
REPO_INPUT="${SPANNERPLAN_REPO:-apstndb/spannerplan-rs}"
if [[ "$REPO_INPUT" == https://github.com/* ]]; then
  REPO="${REPO_INPUT#https://github.com/}"
else
  REPO="$REPO_INPUT"
fi
GIT_REPO_URL="${SPANNERPLAN_GIT_REPO_URL:-https://github.com/${REPO}}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${TAG#v}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

ASSETS="$WORK/assets"
mkdir -p "$ASSETS"
echo "==> Downloading published assets ($REPO@$TAG)"
gh release download "$TAG" --repo "$REPO" --dir "$ASSETS" --clobber \
  --pattern 'SHA256SUMS.txt' \
  --pattern 'spannerplan-ffi-*.tar.gz' \
  --pattern 'spannerplan-ffi-*.zip' \
  --pattern "spannerplan-core-${VERSION}.tgz" \
  --pattern "spannerplan-cli-${VERSION}.tgz"

echo "==> Verifying published checksums"
bash "$ROOT/scripts/verify-release-checksums.sh" "$ASSETS/SHA256SUMS.txt"

echo "==> Rust git dependency ($TAG)"
mkdir -p "$WORK/rust-consumer/src"
cat > "$WORK/rust-consumer/Cargo.toml" <<EOF
[package]
name = "consumer"
version = "0.0.0"
edition = "2021"

[dependencies]
spannerplan = { git = "$GIT_REPO_URL", tag = "$TAG" }
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

case "$(uname -s):$(uname -m)" in
  Linux:x86_64)
    TARGET="x86_64-unknown-linux-gnu"
    ARCHIVE="spannerplan-ffi-${VERSION}-${TARGET}.tar.gz"
    LIB_NAME="libspannerplan_ffi.so"
    ;;
  Darwin:arm64)
    TARGET="aarch64-apple-darwin"
    ARCHIVE="spannerplan-ffi-${VERSION}-${TARGET}.tar.gz"
    LIB_NAME="libspannerplan_ffi.dylib"
    ;;
  Darwin:x86_64)
    TARGET="x86_64-apple-darwin"
    ARCHIVE="spannerplan-ffi-${VERSION}-${TARGET}.tar.gz"
    LIB_NAME="libspannerplan_ffi.dylib"
    ;;
  MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64)
    TARGET="x86_64-pc-windows-msvc"
    ARCHIVE="spannerplan-ffi-${VERSION}-${TARGET}.zip"
    LIB_NAME="spannerplan_ffi.dll"
    ;;
  *)
    echo "unsupported host triple: $(uname -s)-$(uname -m)" >&2
    exit 1
    ;;
esac

FFI_DIR="$WORK/ffi"
mkdir -p "$FFI_DIR"
echo "==> Extracting published FFI archive: $ARCHIVE"
if [[ "$ARCHIVE" == *.tar.gz ]]; then
  tar -xzf "$ASSETS/$ARCHIVE" -C "$FFI_DIR"
else
  unzip -q "$ASSETS/$ARCHIVE" -d "$FFI_DIR"
fi
test -s "$FFI_DIR/$LIB_NAME"
export SPANNERPLAN_FFI_LIB="$FFI_DIR/$LIB_NAME"

echo "==> Python git install with published FFI"
pip install -q "spannerplan @ git+${GIT_REPO_URL}@${TAG}#subdirectory=bindings/python"
python3 -c "
from pathlib import Path
from spannerplan import render_tree_table_json
plan = Path('$ROOT/testdata/reference/dca.yaml').read_text()
out = render_tree_table_json(plan, 'PLAN', 'CURRENT')
assert out and 'Operator' in out
print('ok: python render', len(out), 'bytes')
"

echo "==> npm ci and published Node/browser consumer verification"
(cd "$ROOT/js" && npm ci)
(cd "$ROOT/js" && \
  SPANNERPLAN_CORE_TARBALL="$ASSETS/spannerplan-core-${VERSION}.tgz" \
  SPANNERPLAN_CLI_TARBALL="$ASSETS/spannerplan-cli-${VERSION}.tgz" \
  bash scripts/verify-release-js-tarballs.sh)

echo "All published consumer smoke tests passed for $REPO@$TAG"
