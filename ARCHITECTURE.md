# Architecture

How the Rust renderer, language bindings, and the reference Go implementation
relate. For algorithm-level porting detail and parity strategy, see
[`DESIGN.md`](DESIGN.md).

## Layer cake

```
┌─────────────────────────────────────────────────────────────────┐
│  Surfaces (thin adapters — no rendering logic)                  │
│  cli · ffi · wasm · @spannerplan/* · bindings/* (FFI)           │
└────────────────────────────┬────────────────────────────────────┘
                             │ decode input → PlanNode[]
                             │ call reference API or CLI runner
┌────────────────────────────▼────────────────────────────────────┐
│  spannerplan (std)                                              │
│  YAML/JSON extract, serde_json, convenience re-exports          │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│  spannerplan-core (no_std + alloc)                              │
│  model, queryplan, plantree, treerender, asciitable,            │
│  scalarappendix, stats, reference                               │
│  optional: serde (JSON model), wire (protobuf decode)           │
└─────────────────────────────────────────────────────────────────┘
```

| Layer | Crate / package | Responsibility |
|-------|-----------------|----------------|
| Core | `spannerplan-core` | All rendering. `no_std` + `alloc`. No I/O. |
| Std | `spannerplan` | Text input (YAML→JSON, shape detection), wires core features (`serde`, `wire`). |
| CLI | `spannerplan-cli` | `rendertree` binary — flag parsing, stdin, exit codes (matches Go CLI). |
| FFI | `spannerplan-ffi` | C ABI (`cdylib`): JSON and protobuf-wire inputs → rendered text. |
| WASM | `spannerplan-wasm` | `wasm-bindgen` exports consumed by JavaScript. |
| JS | `@spannerplan/core`, `@spannerplan/cli` | TypeScript API + npm `rendertree` binary over WASM. |
| FFI bindings | [`bindings/`](bindings/) | Python, Java, .NET, C++, Ruby, PHP — thin wrappers over `spannerplan-ffi`. |

Dependency rule: surfaces depend inward; core never depends on std, CLI, FFI,
WASM, JS, or language bindings.

**FFI vs WASM vs pure Go:** see
[ARCHITECTURE.md — Single source of truth](ARCHITECTURE.md#single-source-of-truth)
for which surface to use. FFI platform matrix, memory rules, and release
artifacts: [bindings/README.md](bindings/README.md).

## Single source of truth

**The Rust `spannerplan-core` pipeline is canonical** for this repository.
Correctness is measured by byte-for-byte parity with
[`github.com/apstndb/spannerplan`](https://github.com/apstndb/spannerplan) (Go)
on shared fixtures (`testdata/`).

| Consumer need | Binding | Why |
|---------------|---------|-----|
| Rust library / embedded | `spannerplan-core` or `spannerplan` | Native, `no_std`-capable, no FFI overhead. |
| C, Python, Ruby, Java, .NET, PHP, C++ | [`bindings/`](bindings/) → `spannerplan-ffi` | Stable C ABI; ship a `.so`/`.dylib`/`.dll`. |
| JavaScript / TypeScript | `spannerplan-wasm` → `@spannerplan/core` | Portable `.wasm`; no native compile per platform. |
| Shell / ops | `spannerplan-cli` or Go `rendertree` | stdin/flags; easy piping. |

Go remains the **reference implementation** used to generate goldens and CLI
parity tests. Rust is the **maintained multi-surface port** in this repo. They
are peers for behavior, not parent/child in code terms.

## Reference API vs CLI path

Two WASM/FFI entry styles exist on purpose:

| Entry | Rust / WASM symbol | Use when |
|-------|-------------------|----------|
| **Reference API** | `render_tree_table_with_config` / `spannerplanRenderTreeTable` | Library callers: plan bytes + mode/format + [`RenderConfig`](schema/render-config.schema.json). Matches Go `RenderTreeTableWithConfig`. |
| **Structured Plantree** | `plantree_rows` / `spannerplanPlantreeRows` | Library callers that need typed pre-order Plantree rows rather than formatted table text. Versioned by [`plantree-rows-v1.schema.json`](schema/plantree-rows-v1.schema.json). |
| **CLI runner** | `spannerplan_cli::run_collecting` / `spannerplanRenderRendertree` | Reproduce `rendertree` flag semantics exactly (help text, usage errors, profile column behavior). Node-only in JS (`renderRendertree`). |

All three funnel into the same Plantree processing core; the CLI path adds flag
parsing and process-style I/O. Prefer the **structured Plantree API** when a
consumer needs rows, the **reference API** when it needs the formatted table,
and the **CLI path** only when matching shell tool behavior.

Shared config shape: [`schema/render-config.schema.json`](schema/render-config.schema.json)
(camelCase JSON, decoded by Rust `RenderConfig`, FFI `config_json`, WASM
`config` argument, and TypeScript `RenderConfig`).

## JavaScript / WASM

WASM is built from `crates/spannerplan-wasm` via
[`scripts/build-wasm.sh`](scripts/build-wasm.sh) or `cd js && npm run build:wasm`.
`@spannerplan/core` loads two artifacts:

| Artifact | Features | Used by |
|----------|----------|---------|
| `wasm/` (browser slim) | `wire` only | `wasm-pack --target web`; async `@spannerplan/core/browser` initialization through a package-relative asset URL |
| `wasm-node/` (node full) | `yaml`, `wire`, `cli` | Node.js / `renderRendertree` |

Browser builds omit `serde_yaml_ng` and the CLI; YAML text is parsed in
JavaScript (`yaml` npm) before calling the slim WASM renderer. Node keeps the
full Rust extract path for YAML stdin and `rendertree` CLI parity.

Both accept JSON objects and protobuf wire bytes (`Uint8Array`). Size matrix:
`scripts/measure-wasm-sizes.sh`.

| Package | Role |
|---------|------|
| `@spannerplan/core` | `renderTreeTable`, `renderTreeTableWire`, `plantreeRows`, `plantreeRowsWire`, types |
| `@spannerplan/cli` | `rendertree` npm binary (stdin + flags → WASM CLI path) |

Details (build prerequisites, input shapes, browser bundler, API examples):
[`js/README.md`](js/README.md),
[`js/packages/spannerplan/README.md`](js/packages/spannerplan/README.md).
Browser demo: [`js/examples/rendertree-web`](js/examples/rendertree-web).

## Go positioning (recommendation)

**Keep [`github.com/apstndb/spannerplan`](https://github.com/apstndb/spannerplan) as a pure Go reference implementation.** Do not require CGO, Rust, or WASM in the Go module.

- **Pure Go** — zero native deps; idiomatic for Spanner tooling in Go; goldens and CLI parity tests in this repo shell out to Go `rendertree`.
- **Rust (`spannerplan-rs`)** — `no_std` core, C FFI, WASM, native CLI for embedded and non-Go/non-JS stacks.
- **JavaScript (`@spannerplan/*`)** — same renderer in browsers and Node without native addons.

Rust and JS are **parity-tested alternatives**, not replacements for the Go
module's identity as the idiomatic Spanner-plan tool in Go. Avoid maintaining
a second independent renderer in Go; land behavioral changes in one canonical
algorithm (Rust core here; Go reference until Rust fully owns spec) and prove
equivalence with parity tests.

## `rendertree` by language

Full CLI and sample paths (Rust, Go, Node, browser, C++, and FFI sample CLIs):
[bindings/README.md — `rendertree` samples by language](bindings/README.md#rendertree-samples-by-language).

## Repository map (quick)

```
spannerplan-rs/
├── crates/spannerplan-core/   # no_std renderer
├── crates/spannerplan/        # std input layer
├── crates/spannerplan-cli/    # rendertree binary
├── crates/spannerplan-ffi/    # C ABI + spannerplan.h
├── crates/spannerplan-wasm/   # wasm-bindgen exports
├── js/                        # @spannerplan/* workspace + examples
├── bindings/                  # FFI wrappers (see bindings/README.md)
├── schema/                    # shared JSON schemas (RenderConfig, …)
├── scripts/build-wasm.sh      # repo-root WASM build entry
├── testdata/                  # fixtures + Go-derived goldens (see testdata/README.md)
├── lab/genrsgolden/           # golden regeneration docs (harness lives in Go repo)
├── DESIGN.md                  # porting spec (algorithms, parity)
└── ARCHITECTURE.md            # this file
```
