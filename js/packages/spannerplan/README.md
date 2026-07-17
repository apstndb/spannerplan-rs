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
- **Node vs browser** — Node loads `wasm-node/` (full: yaml + wire + cli, sync
  init); browsers use slim `wasm/` via `@spannerplan/core/browser` (wire +
  JSON renderer, async init). Browser YAML is parsed in JavaScript (`yaml` npm),
  not in WASM.
- **Main entry sync/async** — `renderTreeTable` from `@spannerplan/core` returns
  a result synchronously on Node and a `Promise` in browsers. Use
  `@spannerplan/core/browser` when you always want async/await.
- **Bundler** — browser builds need a bundler that resolves package-relative
  `new URL(..., import.meta.url)` assets (Vite, Webpack 5+, etc.).
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
| Node.js     | yes (WASM) | yes     | yes         | yes                            |
| Browser     | yes (JS `yaml`) | yes | yes    | yes                            |

Node passes YAML/JSON text to full WASM (`serde_yaml_ng`). Browser parses
YAML/JSON text with the `yaml` package, then sends a JSON object to slim WASM.

### Types

- `RenderMode`: `AUTO` | `PLAN` | `PROFILE`
- `Format`: `TRADITIONAL` | `CURRENT` | `COMPACT`
- `RenderConfig`: `wrapWidth`, `hangingIndent`, `printSections`, scalar-var flags, etc.

## Structured Plantree rows

`plantreeRows` exposes the renderer's pre-order Plantree rows as typed data;
it does not parse the formatted table. The Node main entry is synchronous (or
a Promise in browser-like hosts), while the browser entry is always async.

```ts
import { plantreeRows, plantreeRowsOrThrow } from "@spannerplan/core";

const response = await plantreeRows(plan, "CURRENT", { wrapWidth: 80 });
if ("error" in response) throw new Error(response.error);
for (const row of response.rows) {
  console.log(row.nodeId, row.nodeText, row.scalarChildLinks);
}

const rows = await plantreeRowsOrThrow(plan);
```

The success envelope is `{ contractVersion: 1, rows }`; failures are
`{ error }`. Each row has `nodeId`, `treePart`, `nodeText`, `displayName`,
`predicates`, and ordered scalar child links. A child link includes its raw
fields plus `isPredicate`, classified from the query-plan API. `PlantreeConfig`
is deliberately narrow: `wrapWidth`, `hangingIndent`, and
`disallowUnknownStats` only. See
[`schema/plantree-rows-v1.schema.json`](../../../schema/plantree-rows-v1.schema.json).

`plantreeRowsWire` accepts protobuf wire bytes. Import the browser entry when
you always want a Promise:

```ts
import { plantreeRows } from "@spannerplan/core/browser";
```

### Browser / bundler

```ts
import { renderTreeTable } from "@spannerplan/core/browser";
```

Requires a bundler that resolves package-relative
`new URL(..., import.meta.url)` assets (Vite, Webpack 5+, etc.).

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
`wasm32-unknown-unknown` Rust target. The script builds slim `wasm/` (web,
`wire` only) and full `wasm-node/` (`yaml`, `wire`, `cli`) from
`crates/spannerplan-wasm`. Compare sizes with `scripts/measure-wasm-sizes.sh`.

## Tests

```bash
npm test -w @spannerplan/core
```

Golden parity: `testdata/reference/dca.yaml` → `testdata/golden/dca_plan_current.txt`.
