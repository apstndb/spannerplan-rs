# @spannerplan/core

JavaScript/TypeScript library for rendering Cloud Spanner query plans as ASCII
tables. WASM-backed; byte-for-byte parity with the Rust/Go reference renderer on
shared fixtures.

## Requirements & caveats

- **WASM, not FFI** — this package ships prebuilt `.wasm` from
  `crates/spannerplan-wasm`; no `libspannerplan_ffi` or native addon per
  platform. Contrast with Python/Java/.NET bindings under [`bindings/`](../../bindings/).
- **Not a pure-JavaScript renderer** — logic runs in WebAssembly; the npm
  package is a thin TypeScript loader over `wasm-bindgen` glue.
- **Node vs browser** — Node loads `wasm-node/` (sync init); browsers use
  `wasm/` via `@spannerplan/core/browser` (async init). Both accept YAML/JSON
  text, objects, and wire bytes; YAML is parsed in WASM, not in JavaScript.
- **Main entry sync/async** — `renderTreeTable` from `@spannerplan/core` returns
  a result synchronously on Node and a `Promise` in browsers. Use
  `@spannerplan/core/browser` when you always want async/await.
- **Bundler** — browser builds need a bundler that imports `.wasm` (Vite,
  Webpack 5+, etc.).
- **Build from source** — changing the renderer requires Rust +
  `wasm32-unknown-unknown` + `wasm-pack` (`npm run build:wasm`).

See also: [ARCHITECTURE.md](../../ARCHITECTURE.md#javascript--wasm-consumption).

## Install

From a [GitHub Release](https://github.com/apstndb/spannerplan-rs/releases) tarball
(WASM prebuilt; no Rust toolchain):

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

See [`DISTRIBUTION.md`](../../../DISTRIBUTION.md). This package is not published
to npmjs.org.

From this monorepo:

```bash
cd js && npm install && npm run build -w @spannerplan/core
```

## API (reference renderer)

Mirrors Go `RenderTreeTableWithConfig` / Rust `render_tree_table_with_config`:

```ts
import { renderTreeTable, renderTreeTableOrThrow } from "@spannerplan/core";

const yaml = await readFile("plan.yaml", "utf8");
const { output } = await renderTreeTable(yaml, "PLAN", "CURRENT");
// or
const table = await renderTreeTableOrThrow(yaml, "PLAN", "CURRENT", {
  wrapWidth: 80,
  hangingIndent: true,
});
```

### Input shapes

| Environment | YAML | JSON string | JSON object | Protobuf wire (`Uint8Array`) |
|-------------|------|-------------|-------------|--------------------------------|
| Node.js     | yes  | yes         | yes         | yes                            |
| Browser     | yes  | yes         | yes         | yes                            |

YAML and JSON text are passed to WASM; parsing uses Rust `serde_yaml_ng`.

### Types

- `RenderMode`: `AUTO` | `PLAN` | `PROFILE`
- `Format`: `TRADITIONAL` | `CURRENT` | `COMPACT`
- `RenderConfig`: `wrapWidth`, `hangingIndent`, `printSections`, scalar-var flags, etc.

### Browser / bundler

```ts
import { renderTreeTable } from "@spannerplan/core/browser";
```

Requires a bundler that can import `.wasm` files (Vite, Webpack 5+, etc.).

## `rendertree` CLI path (Node only)

For Go/Rust `rendertree` CLI semantics (Latency column in profile mode, etc.):

```ts
import { renderRendertree } from "@spannerplan/core";

const result = renderRendertree(stdinBytes, ["-mode", "plan"]);
```

Prefer the [`@spannerplan/cli`](../cli) package for a shell binary.

## Build WASM glue

```bash
npm run build:wasm -w @spannerplan/core
```

Requires [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) and the
`wasm32-unknown-unknown` Rust target. The script builds both `wasm/` (bundler)
and `wasm-node/` (Node.js) outputs from `crates/spannerplan-wasm`.

## Tests

```bash
npm test -w @spannerplan/core
```

Golden parity: `testdata/reference/dca.yaml` → `testdata/golden/dca_plan_current.txt`.
