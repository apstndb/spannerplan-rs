# JavaScript packages

npm workspaces monorepo for WASM-backed Spanner plan rendering.

**Not FFI:** these packages compile Rust to WebAssembly (`spannerplan-wasm`);
they do not load `libspannerplan_ffi`. For native FFI bindings (Python, Java,
etc.), see [`bindings/`](../bindings/README.md#ffi-bindings-vs-native-implementations).

| Package | Description |
|---------|-------------|
| [`@spannerplan/core`](packages/spannerplan) | Importable library (Node + browser) |
| [`@spannerplan/cli`](packages/cli) | `rendertree` npm binary |

## Quick start

```bash
cd js
npm install
npm run build
npm test

# CLI
npx rendertree -mode plan < ../testdata/reference/dca.yaml
```

## Tests

```bash
npm test
npm run typecheck
```

Golden parity: `reference/dca.yaml` → `golden/dca_plan_current.txt` (reference API)
or `golden/dca_rendertree_plan.txt` (CLI).
Wire parity: `testdata/wire/*.bin` (see [`testdata/wire/README.md`](../testdata/wire/README.md)).

## Prerequisites

- Node.js 20+
- Rust toolchain + `wasm32-unknown-unknown` target
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/)

`npm run build:wasm` (via `@spannerplan/core`) compiles `crates/spannerplan-wasm`.

## Using a release (no local Rust build)

Prebuilt packages are attached to [GitHub Releases](https://github.com/apstndb/spannerplan-rs/releases).
See [`DISTRIBUTION.md`](../DISTRIBUTION.md#javascript--typescript).

```bash
gh release download v0.1.0-alpha.1 --repo apstndb/spannerplan-rs --pattern 'spannerplan-core*.tgz'
npm install ./spannerplan-core-0.1.0-alpha.1.tgz
```

From git (builds WASM from source):

```json
{
  "dependencies": {
    "@spannerplan/core": "github:apstndb/spannerplan-rs#v0.1.0-alpha.1&path:js/packages/spannerplan"
  }
}
```
