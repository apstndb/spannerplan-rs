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
