# spannerplan-rs

Rust port of [apstndb/spannerplan](https://github.com/apstndb/spannerplan): render
Cloud Spanner query plans as ASCII tables and appendices, with byte-for-byte
parity against the Go implementation on shared fixtures.

## Crates

| Crate | Role |
|-------|------|
| `spannerplan-core` | `no_std` renderer (JSON via optional `serde`, protobuf via `wire`) |
| `spannerplan` | `std` helpers: YAML/JSON extract, integration tests |
| `spannerplan-cli` | `rendertree` binary (matches Go CLI table layout) |
| `spannerplan-ffi` | C ABI (`cdylib`) for JSON and wire inputs |
| `spannerplan-wasm` | `wasm-bindgen` entry points |

JavaScript/TypeScript packages (WASM-backed): [`js/`](js/) (`@spannerplan/core`,
`@spannerplan/cli`).

Architecture (layers, bindings, Go vs Rust/JS): [`ARCHITECTURE.md`](ARCHITECTURE.md).
Specification, parity strategy, and implementation notes: [`DESIGN.md`](DESIGN.md).

Shared config schema (`RenderConfig` for Rust, FFI, WASM, JS, bindings):
[`schema/render-config.schema.json`](schema/render-config.schema.json) (example:
[`schema/render-config.example.json`](schema/render-config.example.json)).

Fixtures and byte-for-byte goldens: [`testdata/`](testdata/) (provenance in
[`testdata/README.md`](testdata/README.md)).

Language bindings (Python, Java, .NET, C++, Ruby, PHP) are FFI wrappers over
the Rust cdylib — not pure implementations in those languages. Caveats per
platform: [`bindings/README.md`](bindings/README.md#ffi-bindings-vs-native-implementations).
JavaScript uses WASM (`@spannerplan/core`); Go is pure Go with no native deps.

## Quick start

```bash
cargo test --workspace
cargo run -p spannerplan-cli -- -mode plan < testdata/reference/dca.yaml
```

JavaScript (Node 20+):

```bash
cd js && npm install && npm run build && npm test
npx rendertree -mode plan < ../testdata/reference/dca.yaml
```

Go CLI parity tests in `spannerplan-cli` shell out to `rendertree` when it is on
`PATH`; they are skipped locally with a note if the binary is missing. CI sets
`SPANNERPLAN_GO_PARITY=1`, which makes a missing `rendertree` a hard failure.
Install: `go install github.com/apstndb/spannerplan/cmd/rendertree@v0.1.11`.

## Build gates

```bash
cargo build -p spannerplan-core --no-default-features
cargo build -p spannerplan-core --target thumbv7em-none-eabi --no-default-features
cargo build -p spannerplan-core --target thumbv7em-none-eabi --no-default-features --features wire
```

CI runs these checks on every push (see `.github/workflows/ci.yml`).

## Releases

Tagged releases (`v*`) build FFI artifacts per platform, npm tarballs
(`@spannerplan/core`, `@spannerplan/cli`), and optional crates.io / npmjs.org
publishes when `CARGO_REGISTRY_TOKEN` / `NPM_TOKEN` secrets are configured.
See [`.github/workflows/release.yml`](.github/workflows/release.yml).

```bash
# Rust from git
spannerplan = { git = "https://github.com/apstndb/spannerplan-rs", tag = "v0.1.0-alpha.1" }

# Python from git (FFI library required)
pip install "spannerplan @ git+https://github.com/apstndb/spannerplan-rs@v0.1.0-alpha.1#subdirectory=bindings/python"

# JavaScript from release tarball or local build
npm install ./spannerplan-core-0.1.0-alpha.1.tgz
```

Smoke-test consumer installs locally: `bash scripts/verify-release-consumers.sh v0.1.0-alpha.1`

## Repository map

| Path | Contents |
|------|----------|
| `crates/` | Rust workspace: core, std layer, CLI, FFI, WASM |
| `js/` | `@spannerplan/core` and `@spannerplan/cli` (WASM-backed) |
| `bindings/` | FFI wrappers and sample CLIs — [`bindings/README.md`](bindings/README.md) |
| `schema/` | Shared JSON schemas (`RenderConfig`) |
| `testdata/` | Input fixtures + Go-derived golden outputs |
| `proto/` | Vendored `.proto` subset for the `wire` feature |
| `lab/genrsgolden/` | Docs for regenerating `testdata/golden/` from Go |
