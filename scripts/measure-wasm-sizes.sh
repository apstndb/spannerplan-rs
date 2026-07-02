#!/usr/bin/env bash
# Build spannerplan-wasm feature variants and report .wasm sizes (wasm-pack + wasm-opt).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WASM_CRATE="$ROOT/crates/spannerplan-wasm"
OUT_DIR="$ROOT/lab/wasm-size/out"
mkdir -p "$OUT_DIR"

WASM_PACK="${WASM_PACK:-wasm-pack}"
if ! command -v "$WASM_PACK" >/dev/null 2>&1; then
  WASM_PACK="$HOME/.cargo/bin/wasm-pack"
fi
if ! command -v "$WASM_PACK" >/dev/null 2>&1; then
  echo "wasm-pack not found" >&2
  exit 1
fi

fmt_bytes() {
  local n="$1"
  if command -v numfmt >/dev/null 2>&1; then
    numfmt --to=iec "$n"
  else
    awk -v n="$n" 'BEGIN {
      split("B KB MB GB", u, " ");
      i = 1;
      while (n >= 1024 && i < 4) { n /= 1024; i++ }
      printf "%.1f%s", n, u[i]
    }'
  fi
}

declare -a NAMES=()
declare -a OPT=()
declare -a GZIP=()

build_variant() {
  local name="$1"
  local features="$2"
  local label="$3"
  local out="$OUT_DIR/pack-$name"

  echo "==> wasm-pack: $label (features: ${features:-<none>})"
  rm -rf "$out"
  local pack_args=(build --release --target bundler --out-dir "$out")
  if [[ -n "$features" ]]; then
    "$WASM_PACK" "${pack_args[@]}" -- --no-default-features --features "$features"
  else
    "$WASM_PACK" "${pack_args[@]}" -- --no-default-features
  fi

  local src="$out/spannerplan_wasm_bg.wasm"
  local dst="$OUT_DIR/${name}.wasm"
  cp "$src" "$dst"

  local opt_bytes gzip_bytes
  opt_bytes=$(wc -c < "$dst" | tr -d ' ')
  gzip_bytes=$(gzip -c "$dst" | wc -c | tr -d ' ')

  NAMES+=("$label")
  OPT+=("$opt_bytes")
  GZIP+=("$gzip_bytes")
}

cd "$WASM_CRATE"

# Production targets (see js/packages/spannerplan/scripts/build-wasm.sh)
build_variant "browser-slim" "wire" "browser slim (wire+json, host YAML) [@spannerplan/core wasm/]"
build_variant "node-full" "yaml,wire,cli" "node full (yaml+wire+cli) [@spannerplan/core wasm-node/]"

# Reference variants
build_variant "core-minimal" "" "core minimal (renderer+json only)"
build_variant "core-yaml-only" "yaml" "core (yaml only)"
build_variant "cli-bundle" "yaml,cli" "cli bundle (yaml+cli, no wire)"

printf '\n%-48s %10s %10s\n' "variant" "wasm-opt" "gzip"
printf '%s\n' "--------------------------------------------------------------------------------"

for i in "${!NAMES[@]}"; do
  printf '%-48s %10s %10s\n' \
    "${NAMES[$i]}" \
    "$(fmt_bytes "${OPT[$i]}")" \
    "$(fmt_bytes "${GZIP[$i]}")"
done

printf '\nExact bytes (wasm-pack release, wasm-opt in output):\n'
for i in "${!NAMES[@]}"; do
  printf '  %-48s %d (gzip %d)\n' "${NAMES[$i]}" "${OPT[$i]}" "${GZIP[$i]}"
done

node_full="${OPT[1]}"
browser="${OPT[0]}"
saved=$((node_full - browser))
pct=$((saved * 100 / node_full))
printf '\n  browser slim vs node full: -%d bytes (-%d%%)\n' "$saved" "$pct"

echo
echo "Artifacts: $OUT_DIR/*.wasm"
