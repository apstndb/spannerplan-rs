# spannerplan-rs — Design Document

> **See also:** [`ARCHITECTURE.md`](ARCHITECTURE.md) for the layer cake, binding
> surfaces (CLI / FFI / WASM / JS), shared `RenderConfig` schema, and Go vs
> Rust/JS positioning. This file is the algorithm-level porting spec.

Algorithm-level spec for
[`github.com/apstndb/spannerplan`](https://github.com/apstndb/spannerplan) (Go)
→ Rust, with a `no_std` core and cross-language bindings (C ABI / WASM). For
layer cake, binding surfaces, and repository layout, see
[`ARCHITECTURE.md`](ARCHITECTURE.md).

References to the Go source (`spannerplan@main`) remain for cross-checking
behavior during maintenance.

The reference Go repo was inspected at `HEAD` of `main` (2026-07). File/line
references below are relative to that repo.

---

## 1. Goals and requirements

1. **Faithful port** of the Spanner query-plan rendering pipeline: given a Spanner
   `QueryPlan` (as protojson/JSON, or the enclosing `ResultSetStats` /
   `ResultSet`), produce the same ASCII table + appendix output the Go tool
   produces. Byte-for-byte parity with the Go golden outputs is the acceptance
   bar (see §9).
2. **Core must compile and run under `no_std`** (with `alloc`). This is a hard
   requirement. No filesystem, no I/O, no `std::collections` that require `std`
   (use `alloc::collections`), no panics on untrusted input where avoidable.
3. **Usable as a library** from Rust with an ergonomic typed API.
4. **Bindable from other languages**: a stable C ABI (`cdylib`) that takes either
   JSON text or raw protobuf wire bytes in and returns rendered text out (see
   §8.1), plus a WASM entry point mirroring the Go `examples/wasm/render` wrapper.
5. Ship an equivalent of the `rendertree` CLI.

### Non-goals (initially)

- The Go `--custom` / `--custom-column` / `--custom-file` template-based custom
  columns (uses Go `text/template`). Port last, or offer a reduced version. The
  default two/five-column table and the appendix sections are the priority.
- YAML input parsing in the **core**. YAML→JSON is an I/O/std concern; keep it in
  the `std` layer only (the Go tool accepts YAML via `protoyaml`).
- The `stats` package's PostgreSQL example (`examples/pgexplainjson`).
- `cmd/lintplan`: the Go module marks the whole module EXPERIMENTAL, and
  `lintplan` is a thin heuristic-lint demo on top of the same public API as
  `rendertree`. Not part of the port; nothing in `plantree`/`asciitable`/
  `scalarappendix` depends on it. Revisit only if a user asks for it later.

---

## 2. Reference (Go) architecture map

The Go module is a set of small packages. Port responsibilities map as follows.

| Go package / file | Responsibility | Rust target |
|---|---|---|
| root `queryplan.go` | `QueryPlan` type, parent maps, `NodeTitle` formatting, link classification (`IsVisible`/`IsPredicate`/`IsFunction`/`GetLinkType`), formatting options | `spannerplan-core::queryplan` |
| root `extract.go` | Detect input shape (`queryPlan` / `planNodes` / `stats`) and decode | `spannerplan` (std layer) `extract` |
| `protoyaml/` | YAML→JSON + protojson unmarshal | `spannerplan` (std) `input`; core stays decode-format-agnostic |
| (new) `proto/` + `build.rs` | Vendored `.proto` subset compiled to `no_std`+`alloc` Rust types via `protox`+`prost-build` (protoc-free) | `spannerplan-core::wire` (feature-gated), see §5.3 |
| `plantree/plantree.go` | `ProcessPlan`: builds the visible operator tree, computes predicates, scalar child links, per-row execution stats, then calls the tree renderer to get `TreePart`/`NodeText` | `spannerplan-core::plantree` |
| `treerender/treerender.go` | Generic ASCII tree renderer with wrapping / hanging-indent | `spannerplan-core::treerender` |
| `asciitable/asciitable.go` | Generic ASCII table + appendix renderer (currently backed by `olekukonko/tablewriter`) | `spannerplan-core::asciitable` (reimplemented, no tablewriter) |
| `stats/` | `ExecutionStats` typed struct + JSON round-trip extraction | `spannerplan-core::stats` |
| `internal/scalarappendix/appendix.go` | Predicates / Ordering / Aggregates / Typed / Full appendix sections, scalar-var resolution | `spannerplan-core::scalarappendix` |
| `plantree/reference/` | High-level entry points: `RenderTreeTable*`, `RenderMode`, `Format`, `RenderConfig`, print presets/sections | `spannerplan-core::reference` (typed) + `spannerplan` (std convenience) |
| `cmd/rendertree` | CLI | `spannerplan-cli` (std bin) |
| `examples/wasm/render` | WASM wrapper | `spannerplan-wasm` |
| (new) | C ABI | `spannerplan-ffi` |

Dependency direction (Go): `reference` → `plantree` → {`treerender`, `asciitable`,
`stats`, root `queryplan`} ; `scalarappendix` → {`asciitable`, `plantree`}.
Keep the same layering in Rust.

---

## 3. Rust workspace layout

```
spannerplan-rs/
├── Cargo.toml                # [workspace]
├── ARCHITECTURE.md           # layers, bindings, Go vs Rust/JS positioning
├── DESIGN.md                 # this file (algorithm-level porting spec)
├── crates/
│   ├── spannerplan-core/     # no_std + alloc. The whole rendering pipeline.
│   ├── spannerplan/          # std convenience: JSON/YAML input, extract, re-exports core
│   ├── spannerplan-cli/      # bin: rendertree
│   ├── spannerplan-ffi/      # cdylib, C ABI
│   └── spannerplan-wasm/     # wasm entry (wasm-bindgen)
├── js/                       # @spannerplan/core, @spannerplan/cli (WASM-backed)
├── bindings/                 # FFI wrappers: Python, Java, .NET, C++, Ruby, PHP
├── schema/                   # shared JSON schemas (RenderConfig; see ARCHITECTURE.md)
├── proto/                    # vendored subset of googleapis .proto
├── scripts/build-wasm.sh     # repo-root WASM build entry
└── testdata/                 # Go fixtures + machine-generated goldens (see §9)
```

Rationale for splitting `-core` from `spannerplan`:
- `-core` stays strictly `no_std`; the borrow of `serde` there is optional and
  `default-features = false`.
- JSON parsing (`serde_json`) and YAML (`serde_yaml`/`serde_yaml_ng`) require
  `std`/heap conveniences and belong in the `spannerplan` crate. Keeping them out
  of core guarantees the `no_std` requirement is not accidentally broken.

---

## 4. `no_std` strategy

### 4.1 Core rules
- `crates/spannerplan-core/src/lib.rs` starts with `#![no_std]` and
  `extern crate alloc;`.
- Use `alloc::{string::String, vec::Vec, format, borrow::ToOwned}` and
  `alloc::collections::BTreeMap` (NOT `HashMap`, which is `std`-only). Order
  matters for output determinism anyway (see §4.3), so `BTreeMap` is a feature
  not a limitation.
- No `println!`, no `std::io`. Rendering functions return `String` /
  `Result<String, RenderError>`.
- Error type: a plain `enum RenderError` implementing `core::fmt::Display` and
  `core::fmt::Debug`. Do **not** rely on `std::error::Error` in core; provide a
  `std`-gated impl in the `spannerplan` crate, or gate it behind a `std` feature
  in core (`#[cfg(feature = "std")] impl std::error::Error`).

### 4.2 Dependencies allowed in core (all `no_std`-capable)
- `unicode-width` (display width; `no_std` by default) — replaces the width part
  of `go-tabwrap`/`clipperhouse/displaywidth`.
- `unicode-segmentation` (grapheme clusters; `no_std`) — replaces the grapheme
  iteration part of `go-tabwrap`/`uax29`. Needed so truncation never splits a
  grapheme cluster (matches Go behavior which is grapheme-aware).
- `serde` with `derive`, `default-features = false`, `features = ["derive", "alloc"]`
  — **optional**, behind a `serde` feature. This lets the typed model derive
  `Deserialize` so any JSON lib the consumer picks can populate it, without
  pulling `std` into core.
- `prost` + `prost-types`, `default-features = false` (drops the `std` feature,
  keeping `alloc`) — **optional**, behind a `wire` feature. See §5.3 for why
  this is preferred over hand-rolling a wire decoder.
- Nothing else. No regex crate in core (see §6.8 for the one regex-shaped use,
  hand-written instead of pulling in `regex`).

### 4.3 Determinism / output-order caveats (IMPORTANT)
The Go code iterates Go maps in `NodeTitle` and then **sorts** the result:
- `queryplan.go` `NodeTitle` collects `labels` and `fields` from
  `metadata.GetFields()` (a Go map, random order) and then does
  `sort.Strings(labels); sort.Strings(fields)` before joining. Rust: use
  `BTreeMap` for metadata or sort a `Vec` — either yields sorted, deterministic
  output identical to Go. **Use sorted order for `fields` and `labels`.**
- `scalarappendix` builds groups in **first-seen order** (it uses a map only to
  find the group index, but appends groups to a slice in encounter order and
  preserves `ChildLinks` order within a group). Do NOT sort these; preserve
  child-link order. Model with a `Vec<(String, Vec<String>)>` + a lookup map.

Getting these two orderings right is the most common source of golden-output
diffs. See §6.2 and §6.7.

---

## 5. Input format and data model

### 5.1 Three decode paths, one internal model
The Go tool only ever sees **protojson** (`protojson.Unmarshal` in
`protoyaml/protoyaml.go`), because the CLI reads YAML/JSON files. A Rust library
has a wider set of realistic callers, so this port supports three ways to get a
`spannerplan_core::model::PlanNode` tree, all converging on the same internal
model so `queryplan.rs`/`plantree.rs`/etc. are decode-path-agnostic:

| Path | Typical caller | Where it lives | Cost |
|---|---|---|---|
| **protojson (JSON)** | CLI reading a YAML/JSON plan file; browser/WASM caller with a `JSON.stringify`'d plan; anything scripting-language-shaped | `spannerplan` (std), `serde_json` | one parse pass |
| **protobuf wire (binary)** | A caller holding raw bytes from a Spanner gRPC response, or from another language's protobuf library, passed across an FFI boundary | `spannerplan-core` (no_std+alloc), feature `wire`, §5.3 | one decode pass, no text round-trip |
| **typed struct interop** | A Rust caller that already has a decoded protobuf message in memory (e.g. from a Spanner client crate) | `spannerplan` (std), feature-gated `From`/`TryFrom` impls, §5.4 | zero parsing — plain struct mapping |

Pick the path that matches what the caller already has; don't force everyone
through JSON. §8.1 exposes both the JSON and wire paths at the FFI boundary.

### 5.2 protojson (JSON) decode
protojson is plain JSON with a few encoding quirks to handle in the serde model:
- `camelCase` field names by default (Spanner API emits camelCase), but protojson
  **also accepts the original proto field names** (`snake_case`). Decode should
  accept both (serde `alias`).
- `google.protobuf.Struct` (the `metadata` and `executionStats` fields) is encoded
  as an ordinary JSON object. Values are `google.protobuf.Value` = JSON
  scalar/array/object.
- Enums as strings: `kind` is `"RELATIONAL"` / `"SCALAR"` / `"METADATA"` (also
  accept the numeric form and `"KIND_UNSPECIFIED"`).
- `int32` fields (`index`, `childIndex`) may appear as JSON number or string in
  protojson; accept both (serde with a numeric-or-string helper).

This path lives entirely in the `spannerplan` std crate (§5.6 is the model it
populates); `spannerplan-core` itself never needs `serde_json`.

### 5.3 Proto wire (binary) decode — `prost` + `protox`, no `protoc` needed
For callers that already hold protobuf-encoded bytes (a gRPC response, or bytes
handed across an FFI/WASM boundary from another language's protobuf stack),
parsing them by round-tripping through text is wasted work. `spannerplan-core`
can decode the wire format directly, and still satisfy the `no_std`+`alloc`
requirement:

- **Codegen**: use [`prost`](https://github.com/tokio-rs/prost) for the
  generated message types, and [`protox`](https://github.com/andrewhickman/protox)
  instead of a `protoc` binary. `protox` is a pure-Rust `.proto` compiler that
  produces a `FileDescriptorSet` prost-build can consume
  (`prost_build::Config::compile_fds`), so the whole codegen pipeline is
  dependency-free at build time — no `protoc` install, matching the original
  "lighter than pulling in the full generated client" motivation for this
  project. Vendor a trimmed `.proto` subset in `proto/` (just the messages in
  §5.5, copied from `googleapis/googleapis`, Apache-2.0 — keep attribution) so
  builds are reproducible without a network fetch.
- **`no_std`+`alloc`**: both `prost` and `prost-types` support this by disabling
  their default `std` feature (`default-features = false`) while keeping `alloc`
  — confirmed current for `prost` 0.14.x. Generated code then only depends on
  `alloc::{vec::Vec, string::String}`, matching the rest of the core.
- **Why `prost` over hand-rolling a decoder or using `micropb`**: the trickiest
  part of this schema is `google.protobuf.Struct`/`Value` (a `oneof` plus
  `map<string, Value>`, which is itself wire-encoded as repeated `MapEntry`
  submessages) — that's what `metadata` and `execution_stats` are. `prost-types`
  already ships correct, tested `Struct`/`Value`/`ListValue`/`NullValue`
  generated types, so this complexity doesn't need to be reimplemented. Configure
  codegen with `.extern_path(".google.protobuf", "::prost_types")` so the
  generated Spanner messages reference `prost_types::Struct`/`Value` directly
  instead of regenerating them. `micropb` was considered — it's `no_std` **and**
  allocator-free, which is a stronger guarantee than this project needs (we
  already require `alloc` throughout for `String`/`Vec`/`BTreeMap`), and its
  `oneof`/map/well-known-type support is explicitly limited compared to `prost`'s.
  Given the choice between "no_alloc but weaker WKT/oneof/map support" and
  "alloc-only but full WKT/oneof/map support for free", `prost` fits this
  project's actual constraint (`no_std`+`alloc`, not `no_std`+`no_alloc`) better.
  Re-evaluate only if a future no-allocator target becomes a real requirement.
- **Layering**: the `prost`-generated types are an internal decode detail, not
  part of the public API. `wire.rs` (feature-gated) provides `From<generated::
  PlanNode> for model::PlanNode` (and siblings) so the rest of the pipeline only
  ever sees the crate's own model, identical to what the JSON path produces. This
  also means prost's `Option<Box<T>>`-heavy generated shape and its version don't
  leak into the crate's public API or semver surface.
- If the maintainer wants a fully protoc-free but also `protox`/`prost`-free
  build (e.g. to minimize the build-dependency tree further), hand-rolling a
  minimal wire decoder for just this message set remains a fallback — but given
  `Struct`/`Value` complexity, it's more code and more risk of subtle wire-format
  bugs than adopting `prost`+`protox`. See §14 for this as an open decision if
  priorities change.
- **Build-dependency cost:** `protox` and `prost-build` are unconditional
  `[build-dependencies]` on `spannerplan-core` because Cargo build scripts cannot
  gate their own compilation on features. The `build.rs` early-returns when the
  `wire` feature is off, but the compiler stack is still compiled. Committing
  pre-generated prost output (option 1 in maintenance notes) would remove this
  cost; until then, embedded/`--no-default-features` consumers pay the compile
  time once per clean build.

### 5.4 Direct struct interop (Rust-native callers)
For Rust callers that already hold a decoded protobuf message — e.g. from a
Spanner client crate — add optional `From`/`TryFrom` conversions in the `std`
`spannerplan` crate (feature-gated per source crate, e.g. `interop-google-cloud-rust`)
that map that crate's generated `QueryPlan`/`PlanNode` struct fields directly into
`spannerplan_core::model`. No bytes are (re)serialized. Whether such a crate's
generated types are themselves `prost`-based (which would make the mapping a
near-free field-for-field copy, or even allow reusing `wire.rs`'s `From` impls
almost verbatim) is worth checking against the specific client crate during
implementation — different Spanner client generators may use different
protobuf backends. Treat this as a nice-to-have layered on top of §5.2/§5.3,
not a blocker for the initial port.

### 5.5 Proto shape (google.spanner.v1) — the subset we use
From `google/spanner/v1/query_plan.proto` and `result_set.proto`:

```
QueryPlan { repeated PlanNode plan_nodes = 1; }

PlanNode {
  int32 index = 1;
  Kind  kind  = 2;                         // RELATIONAL | SCALAR | (KIND_UNSPECIFIED)
  string display_name = 3;
  repeated ChildLink child_links = 4;
  ShortRepresentation short_representation = 5;
  google.protobuf.Struct metadata = 6;
  google.protobuf.Struct execution_stats = 7;

  message ChildLink { int32 child_index = 1; string type = 2; string variable = 3; }
  message ShortRepresentation { string description = 1; map<string,int32> subqueries = 2; }
}

ResultSetStats { QueryPlan query_plan = 1; google.protobuf.Struct query_stats = 2; ... row_count ... }
ResultSet { StructType-bearing metadata = 1; repeated ListValue rows = 2; ResultSetStats stats = 3; }
```

### 5.6 Rust model (`model.rs`)
```rust
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PlanNode {
    #[cfg_attr(feature="serde", serde(default, deserialize_with = "de_i32_str_or_num"))]
    pub index: i32,
    #[cfg_attr(feature="serde", serde(default))]
    pub kind: Kind,
    #[cfg_attr(feature="serde", serde(default))]
    pub display_name: String,
    #[cfg_attr(feature="serde", serde(default))]
    pub child_links: Vec<ChildLink>,
    #[cfg_attr(feature="serde", serde(default))]
    pub short_representation: Option<ShortRepresentation>,
    #[cfg_attr(feature="serde", serde(default))]
    pub metadata: Metadata,        // BTreeMap<String, MetadataValue>
    #[cfg_attr(feature="serde", serde(default))]
    pub execution_stats: Option<MetadataStruct>, // kept as raw struct; see stats.rs
}

pub enum Kind { Unspecified, Relational, Scalar }   // Default = Unspecified

pub struct ChildLink { pub child_index: i32, pub r#type: String, pub variable: String }

pub struct ShortRepresentation { pub description: String /*, subqueries: BTreeMap<String,i32> */ }
```

Metadata value type: the Go code only ever reads `.GetStringValue()` on metadata
values (see `queryplan.go` `NodeTitle`). protojson `Value` may be string, number,
bool, null, object, list. Model as:
```rust
pub enum MetadataValue { Null, Bool(bool), Number(f64), String(String), List(Vec<MetadataValue>), Struct(Metadata) }
impl MetadataValue { pub fn as_str(&self) -> &str { if let String(s)=self {s} else {""} } }
```
`Metadata = BTreeMap<String, MetadataValue>` (sorted keys → deterministic; see §4.3).

**Getter semantics must mirror Go's `Get*` nil-safety**: every accessor returns a
zero value when absent (`""` for strings, `0` for ints, empty slice). Implement
these as helper methods so the porting of algorithms is mechanical.

### 5.7 Input detection (std layer, ports `extract.go`)
`ExtractQueryPlan(bytes) -> (ResultSetStats, Option<StructType>)`:
1. (std) If input looks like YAML, convert YAML→JSON (serde_yaml → serde_json
   Value). If already JSON, use directly.
2. Peek top-level keys: if `queryPlan` present → decode `ResultSetStats`; else if
   `planNodes` present → wrap into `ResultSetStats{ query_plan }`; else if `stats`
   present → decode `ResultSet`, return `.stats` and `.metadata.rowType`; else
   error `"unknown input format"`. (Mirror `extract.go` exactly.)
3. `DiscardUnknown` = true: unknown fields must be ignored (serde
   `#[serde(default)]` + no `deny_unknown_fields`).

---

## 6. Algorithm porting details (per module)

### 6.1 `queryplan.rs` — QueryPlan construction and validation
Port `queryplan.go` `New`:
- Reject empty `plan_nodes` (`ErrEmptyPlanNodes`).
- Require `plan_nodes[i].index == i` (`ErrPlanNodeIndexMismatch`). Nil-node case
  doesn't exist in Rust (`Vec<PlanNode>` not `Vec<Option>`); if decoding can
  produce a "missing" node, guard it.
- Validate every `child_link.child_index` in range (`ErrChildLinkIndexOutOfRange`).
- Build `parent_map: BTreeMap<i32,i32>` (child_index → parent index) and
  `parent_links_map: BTreeMap<i32, Vec<ResolvedParentLink>>` in traversal order.

Accessors (all direct ports): `plan_nodes`, `get_node_by_index`,
`get_node_by_child_link` (nil link → root node index 0... note: Go
`link.GetChildIndex()` on nil returns 0, so a `None` link resolves to node 0 =
root), `get_parent_node_by_child_index/link`, `parent_links` (clone),
`resolve_child_link`.

**Link classification** (drives what is rendered):
- `is_visible(link)`: `node.kind == Relational || link.r#type == "Scalar"`. A
  `None` link (root) → node 0 is Relational → visible. (`queryplan.go` IsVisible.)
- `visible_child_links(node)`: filter children by `is_visible`.
- `is_function(link)`: `get_node_by_child_link(link).display_name == "Function"`.
- `is_predicate(link)`: see `queryplan.go`:
  - if `link.type == "Search Predicate"` → true iff child kind == Scalar;
  - else if not `is_function` → false;
  - else true iff `link.type` ends with `"Condition"` OR `== "Split Range"`.
- `get_link_type(link)`: if `link.type != ""` return it; else if parent
  `display_name` ends with `"Apply"` AND this link is the parent's **first** child
  link (by child_index) → return `"Input"`; else `""`. (Apply-input workaround.)

### 6.2 `queryplan.rs` — `NodeTitle` (most detail-sensitive function)
Direct port of `queryplan.go` `NodeTitle`. Options struct:
```
ExecutionMethodFormat { Raw(0), Angle(1) }   // parse RAW|ANGLE
TargetMetadataFormat   { Raw(0), On(1) }      // parse RAW|ON
KnownFlagFormat        { Raw(0), Label(1) }   // parse RAW|LABEL (FullScanFormat = alias)
compact: bool
hide_metadata: bool
inline_stats_fn: Option<fn(&PlanNode)->Vec<String>>
```
`sep = if compact {""} else {" "}`.

Steps (keep the exact field skipping logic):
1. `execution_method = metadata["execution_method"].as_str()`.
2. `target` = first non-empty of `metadata[k]` for k in
   `["scan_target","distribution_table","table"]`.
3. `operator = join_if_not_empty(" ", [call_type, iterator_type,
   trim_suffix(scan_type,"Scan"), display_name,
   if target_fmt==On && target!="" {"on "+target} else {""}])`.
4. `execution_method_part = if exec_fmt==Angle && exec_method!="" {"<"+em+">"} else {""}`.
5. If `!hide_metadata`, iterate metadata keys (SORTED — use BTreeMap iteration)
   and build `labels` + `fields` with this switch (see Go for exact conditions):
   - skip `call_type`, `iterator_type`, `scan_type`, `subquery_cluster_node`;
   - `scan_target`: if target_fmt != Raw skip; else push
     `"{trim_suffix(scan_type,'Scan')}: {value}"` to `fields`;
   - `execution_method`: skip if exec_fmt != Raw;
   - `distribution_table`,`table`: skip if target_fmt != Raw;
   - if target_fmt != Raw and key in target keys → skip (guard at top of loop);
   - known boolean flags (`"Full scan"`, `"split_ranges_aligned"`): if
     known_flag_fmt != Raw → push key to `labels` iff value=="true", continue;
   - else push `"{key}:{sep}{value}"` to `fields`.
6. `inline_stats = inline_stats_fn(node)` if set.
7. `sort(labels); sort(fields);` (Go sorts both).
8. Return `join_if_not_empty(sep, [operator, execution_method_part,
   enclose_if_not_empty("(", concat(labels,fields,inline_stats).join(","+sep), ")")])`.

Helpers: `join_if_not_empty(sep, parts)` filters empties then joins;
`enclose_if_not_empty(open, s, close)` returns "" if s empty else wrapped.

`known_boolean_flag_keys = ["Full scan","split_ranges_aligned"]`,
`target_metadata_keys = ["scan_target","distribution_table","table"]`.

### 6.3 `textwidth.rs` — display width, truncate, fill (replaces `go-tabwrap`)
The Go code uses `tabwrap.StringWidth`, `tabwrap.Truncate(s, width, suffix)`,
`tabwrap.FillLeft(s, n)`, and a `Condition{TrimTrailingSpace:true}`. Reimplement:

```rust
pub fn string_width(s: &str) -> usize;             // sum of grapheme cluster widths
pub fn truncate(s: &str, budget: usize) -> &str;   // longest grapheme prefix with width <= budget; suffix always "" in our usage
pub fn fill_left(s: &str, width: usize) -> String; // right-align: left-pad with spaces to display width
```
Implementation notes:
- Iterate **grapheme clusters** (`unicode-segmentation`); width of a cluster =
  `unicode_width::UnicodeWidthStr::width(cluster)` (handles East Asian wide,
  combining marks → 0). This matches `clipperhouse/displaywidth` closely enough
  for the Spanner fixtures (all ASCII in current goldens, but keep it correct for
  CJK/emoji). Confirm against Go if any wide-char fixture is added.
- `truncate` returns a prefix that does not exceed `budget` columns and never
  splits a grapheme. Our callers always pass suffix `""`.
- The renderer relies on `truncate` returning `""` when even one cluster doesn't
  fit; the wrap loop then force-takes one `char` (see §6.4 `wrap_chunks`).
- `TrimTrailingSpace` is applied by the caller via `trim_end_matches([' ','\t'])`.

Also port `treerender.NewPrefixMetrics` / `MaxWidthForDepth` if needed (only used
by callers computing prefix width; plantree passes `wrap_width` through, so this
may be unnecessary — verify. It is defined in `treerender.go` but `plantree`
computes wrapping via the renderer itself, not `PrefixMetrics`).

### 6.4 `treerender.rs` — generic ASCII tree renderer
Direct port of `treerender/treerender.go`. Generic over a node type via closures
(Rust: take `get_text: impl Fn(&T)->String`, `get_children: impl Fn(&T)->&[/*child*/]`,
or operate on an owned `RenderedNode` tree — simplest is a concrete internal
`RenderedNode` since only plantree uses it. Recommend a concrete tree to avoid
generic lifetime pain; keep the generic `Render`/`RenderTree` only if you want to
mirror the public API).

Types:
- `Style { edge_link, edge_mid, edge_end, edge_separator: String, indent_size: usize }`.
  `default_style()` = `{"|","+-","+-"," ",2}`; `compact_style()` = `{"|","+","+","",0}`.
- `Row { tree_part: String, node_text: String }` with `text()` that zips
  tree-part lines and node-text lines (see `Row.Text` — pads to max line count).
- `ContinuationIndent { Tree(0), Anchor(1) }`.
- `RenderOptions { get_continuation_anchor: Option<Fn(&T)->String>, wrap_width: i64,
  wrap_condition (trim_trailing_space bool), continuation_indent }`.

`styleWidths` precompute: `indent = max(0, indent_size)`,
`w_link/w_mid/w_end/w_sep = string_width(glyph)`,
`seg_has_next = edge_link + " "*indent`, `seg_no_next = " "*(indent + w_link)`.
`segment(has_next)` picks one; `continuation_segment(is_last) = segment(!is_last)`.

`render_tree` walk (preorder), tracking `ancestor_prefix`, `is_last`, `is_root`:
- Determine `last_idx` = index of last non-nil child (Rust: last child).
- Compute `anchor` only if `wrap_width>0 && indent==Anchor && get_anchor set`.
- `render_row(...)` produces the `Row`.
- Recurse: `next = if is_root {ancestor_prefix} else {ancestor_prefix + segment(!is_last)}`.

`render_row`:
- If `wrap_width <= 0`: `tree_part = prefix_lines_from_ancestor(...).join("\n")`,
  `node_text = text`. (No wrapping.)
- Else compute `(first_prefix, continuation_prefix)` via `row_prefixes` and call
  `wrap_row_lines`.

`row_prefixes(ancestor, is_last, is_root)`:
- root → `("","")`.
- else `first = ancestor + edge_for_row(is_last) + edge_separator`,
  `continuation = ancestor + continuation_segment(is_last)`.
- `edge_for_row(is_last) = if is_last {edge_end} else {edge_mid}`.

`wrap_row_lines(text, anchor, first_prefix, cont_prefix, has_children, child_guide=edge_link, wrap_width, cond, cont_indent)`:
- If `cont_indent==Anchor && anchor!="" && text.starts_with(anchor)`:
  `anchor_width = string_width(anchor)`, strip anchor from `text`; else `anchor=""`,
  `anchor_width=0`.
- `first_budget = max(1, wrap_width - string_width(first_prefix) - anchor_width)`.
- `continuation_budget = max(1, wrap_width - string_width(cont_prefix) - anchor_width)`.
- `node_lines = wrap_chunks(text, first_budget, continuation_budget, cond)`; if
  empty → `[""]`; then `node_lines[0] = anchor + node_lines[0]`.
- `tree_lines[0] = first_prefix`; `continuation_tree = cont_prefix + (if anchor_width>0
  { hanging_indent_padding(anchor_width, has_children, child_guide, cond) } else {""})`;
  fill `tree_lines[1..] = continuation_tree`.

`wrap_chunks(text, first_budget, cont_budget, cond)`:
- Split on `\n`. For each raw line: if empty push `""` and set budget=cont_budget.
  Else loop: `raw_chunk = truncate(raw_line, budget)`; if `raw_chunk==""` force-take
  one UTF-8 char (`raw_line[..char_len]`); `chunk = if trim_trailing_space
  {raw_chunk.trim_end_matches([' ','\t'])} else {raw_chunk}`; push `chunk`; advance
  `raw_line = &raw_line[raw_chunk.len()..]`; `budget = cont_budget`.
  (Note: advance by **byte length of the untrimmed `raw_chunk`**, not the trimmed
  chunk — matches Go `rawLine = rawLine[len(rawChunk):]`.)

`hanging_indent_padding(anchor_width, has_children, child_guide, cond)`:
- `anchor_width<=0` → `""`.
- `!has_children || child_guide==""` → `" "*anchor_width`.
- else guide = child_guide; if `string_width(guide) > anchor_width` truncate to
  fit (if becomes "" → spaces); return `guide + " "*(anchor_width - width(guide))`.

`prefix_lines_from_ancestor(ancestor, text, is_last, is_root)` (no-wrap path):
- split text on `\n`; if root → all empty prefixes; else line0 =
  `ancestor + edge_for_row(is_last) + edge_separator`, rest =
  `ancestor + continuation_segment(is_last)`.

### 6.5 `stats.rs` — execution stats extraction
Port `stats/types.go` + `stats/extract.go`. The Go `Extract` does a JSON
round-trip from the `execution_stats` Struct into the typed `ExecutionStats`
struct, optionally erroring on unknown fields.

In Rust, `execution_stats` is already decoded as `MetadataStruct` (a
`BTreeMap<String, MetadataValue>`) OR you can deserialize it straight into the
typed struct. Two options:
- **(A) Deserialize twice**: keep `execution_stats` as raw JSON `Value` in the
  model, then `serde_json::from_value` into `ExecutionStats` in the std layer.
  Simple but not `no_std`.
- **(B, recommended)** Implement extraction in core from `MetadataStruct` by hand:
  map each known key → `ExecutionStatsValue`. This keeps stats extraction in the
  `no_std` core (the renderer needs `Rows.Total`, `ExecutionSummary.NumExecutions`,
  `Latency.String()`).

`ExecutionStatsValue { unit, total, mean, std_deviation: String, histogram: Vec<..> }`
with `to_string()`: `if unit=="" {total} else {format!("{total} {unit}")}`.
Note: values arrive as JSON — `total`/`mean` may be strings or numbers; coerce to
string preserving the original text where possible (Spanner sends them as strings,
e.g. `"12.25"`, `"386"`). The typed field list (JSON keys) is in `stats/types.go`
(`ExecutionStats`, `ExecutionStatsSummary`, `ExecutionStatsHistogram`). The keys
include spaced names like `"Disk Usage (KBytes)"` and snake_case like
`"cpu_time"`, `"latency"`, `"rows"`, plus `execution_summary` (nested) with
`num_executions`, `checkpoint_time`, `num_checkpoints`, etc.

`disallow_unknown_stats`: when set, an unknown key in `execution_stats` must be an
error (mirrors `dec.DisallowUnknownFields()`). Track known keys and error on
extras when the flag is on. Only the renderer uses `Rows`, `Latency`,
`ExecutionSummary.NumExecutions`, but the round-trip must still validate the whole
object when the flag is on.

### 6.6 `plantree.rs` — `ProcessPlan`
Port `plantree/plantree.go`. Produces `Vec<RowWithPredicates>`.

`RowWithPredicates` fields to keep (the deprecated ones can be dropped or kept;
the appendix code below only needs `id`, `tree_part`, `node_text`, `display_name`,
`predicates`, `execution_stats`, `scalar_child_links`):
```rust
pub struct RowWithPredicates {
    pub id: i32,
    pub tree_part: String,
    pub node_text: String,
    pub display_name: String,
    pub predicates: Vec<String>,
    pub execution_stats: ExecutionStats,
    pub scalar_child_links: Vec<ScalarChildLink>,
}
pub struct ScalarChildLink { pub r#type: String, pub variable: String, pub description: String, pub display_name: String, pub child_index: i32 }
impl RowWithPredicates {
    pub fn text(&self) -> String;         // treerender Row{tree_part,node_text}.text()
    pub fn format_id(&self) -> String;    // (if !predicates.empty(){"*"}else{""}) + id
    pub fn tree_part_lines(&self) -> Vec<&str>;
}
```

Options (`plantree.Option`): `disallow_unknown_stats`, `queryplan_options` (Vec of
NodeTitle options), `style` (default/compact), `compact`, `hanging_indent`,
`wrap_width: Option<i64>`, plus deprecated `continuation_indent`. `EnableCompact()`
sets compact style + compact NodeTitle + compact treerender style. Validate:
`wrap_width < 0` → error.

`build_rendered_tree(qp, link, opts) -> Option<RenderedNode>` (recursive):
- if `!qp.is_visible(link)` → None.
- `sep = if !compact {" "} else {""}`.
- `node = qp.get_node_by_child_link(link)`; guard index >= 0.
- `link_type = qp.get_link_type(link)`;
  `continuation_anchor = if link_type!="" {"["+link_type+"]"+sep} else {""}`.
- `node_text = continuation_anchor + NodeTitle(node, queryplan_options)`.
- `predicates`: for each child link `cl` of node where `qp.is_predicate(cl)`:
  push `format!("{}: {}", cl.type, child(cl).short_representation.description)`.
- `scalar_child_links`: resolve all child links, filter `child.kind == Scalar`,
  map to `ScalarChildLink` preserving order.
- `execution_stats = stats::extract(node, disallow_unknown_stats)`.
- children: for each `qp.visible_child_links(node)`, recurse; collect Some.
- Return `RenderedNode { id, continuation_anchor, node_text, display_name,
  predicates, execution_stats, scalar_child_links, children }`.

Then:
- `root = build_rendered_tree(qp, None, opts)`; if None → empty.
- `wrap_width = opts.wrap_width.unwrap_or(0)`.
- `render_rows = treerender::render_tree_with_options(root, style, |n| n.node_text,
   |n| n.children, RenderOptions{ get_continuation_anchor: |n| n.continuation_anchor,
   wrap_width, wrap_condition, continuation_indent: map_hanging_indent(hanging_indent) })`.
- `nodes = collect_preorder(root)`; assert `render_rows.len()==nodes.len()`.
- Zip into `RowWithPredicates`, checking line-count consistency between
  `tree_part` and `node_text` (Go does this and errors otherwise).

### 6.7 `asciitable.rs` — table + appendix (REIMPLEMENT, no tablewriter)
The Go code uses `olekukonko/tablewriter` with `StyleASCII`, `TrimSpace off`,
`HeaderAutoFormat off`, `HeaderAlignment left`, per-column row alignment,
`RowAutoWrap none`. We must reproduce that exact ASCII output.

Target format (from the golden in §2, `reference_test.go`):
```
+-----+----------...----+------+-------+---------------+
| ID  | Operator        | Rows | Exec. | Total Latency |
+-----+----------...----+------+-------+---------------+
|  *0 | Distributed ... |  386 |     1 | 12.25 msecs   |
...
+-----+----------...----+------+-------+---------------+
```
Rules to reproduce (verify against goldens byte-for-byte):
- Column width = max display width over header and all cells in that column.
- Border rows: `+` + `-`*(width+2) per column + `+`.
- Data/header rows: `| ` + padded-cell + ` |` per column, one leading space and
  one trailing space of padding inside each cell (i.e. cell content is placed in a
  field of `width` columns, then surrounded by single spaces).
- **Header is left-aligned** regardless of column alignment
  (`WithHeaderAlignment(AlignLeft)`).
- Row cells use per-column alignment: `ID`=right, `Operator`=left, `Rows`=right,
  `Exec.`=right, `Total Latency`=left (see `reference.go` `spannerTableSpec`).
- `TrimSpace off` + `RowAutoWrap none`: cells are used verbatim; no internal
  trimming, no wrapping (wrapping already happened in treerender, producing
  multi-line `Operator` cells — BUT note: the golden shows single-line rows;
  multi-line cells occur only with `--wrap-width`. tablewriter renders embedded
  `\n` as multiple visual lines within the same table row, each line padded to
  column width. You must replicate multi-line cell rendering: a logical row whose
  tallest cell has N lines occupies N visual lines; shorter cells are blank-padded
  on the extra lines. Check `distributed_cross_apply.yaml` wrapped fixtures for
  the exact multi-line layout.)
- Alignment padding uses **display width** (`string_width`), not byte/char count.

Right-alignment for `ID` with the `*` prefix: `format_id()` returns e.g. `*0`,
`31`, `*31`; right-aligned in a width-3 field → `" *0"`, `" 31"`, `"*31"`. The
golden shows `|  *0 |` and `| *31 |` — consistent with width=3 right alignment
plus the surrounding single spaces.

**Appendix** (`RenderAppendix`, ports `asciitable.go`):
- Collect rows where `items(row)` non-empty; compute `max_id_len` over ALL rows'
  ids (even those without items — Go computes maxIDLength across every row).
- If no row has items → return `""` (no title printed).
- Else print `title` line, then for each row-with-items, for each item line i:
  - `id_part = if i==0 {format!("{}:", id)} else {""}`.
  - `prefix = fill_left(id_part, max_id_len + 1)`.
  - write `" {prefix} {item}\n"` (note leading space, then prefix, then space).
Reproduce spacing exactly; see golden:
```
Predicates(identified by ID):
  0: Split Range: (STARTS_WITH($AlbumTitle, 'T') ...)
  1: Split Range: ...
```
Here `max_id_len` handles two-digit ids (e.g. `31`) → alignment column widens.

### 6.8 `scalarappendix.rs`
Direct port of `internal/scalarappendix/appendix.go`. Sections:
`predicates | ordering | aggregate | typed | full`; presets `basic | enhanced |
full | none`. Parsing rules (`ParseSections`, `ParsePreset`, `ParseSection`,
`ValidateSections`) ported verbatim, including:
- comma-separated = section-list syntax; single token tries preset then section;
- `typed`/`full` cannot combine with others; duplicates rejected;
- empty/blank input → explicit empty list (renders no appendix).
- default (`sections == None`) → `[Predicates]`.

`Render(rows, opts) -> String`:
- resolve sections; build a `scalarLinkResolver` (var→description map) only if
  resolve-vars requested AND ordering/aggregate present.
- For each section, build a part via the appendix renderer:
  - `Full`/`Typed`: title `"Node Parameters(identified by ID):"`, items =
    `scalar_link_lines(row, include, format_raw)` where include = (Full ||
    link.type != ""); `format_raw`: `if variable!="" {"${var}={desc}"} else {desc}`.
  - `Predicates`: title `"Predicates(identified by ID):"`, items = `row.predicates`.
  - `Ordering`: title `"Ordering(identified by ID):"`, include =
    `is_ordering_scalar_link(row, link)`, format = semantic (key desc, optionally
    var, optionally resolved). Ordering include rule:
    Sort/Sort Limit → type=="Key"; Minor Sort/Minor Sort Limit → type in
    {MajorKey, MinorKey}; else false.
  - `Aggregate`: title `"Aggregates(identified by ID):"`, include =
    `row.display_name=="Aggregate" && type in {Key, Agg}`, format semantic
    (for Key resolve like ordering; Agg → plain description).
- Concatenate non-empty parts joined by a single `"\n"` between parts (see Go:
  `if b.Len()>0 { b.WriteString("\n") }`).

`scalar_link_lines(row, include, format)`: group by `link.type` in **first-seen
order** (§4.3), within group preserve order, `join(", ")`, prefix `"{type}: "`
when type non-empty; drop empty groups.

Scalar-var resolution (`--resolve-vars`, `--resolve-vars-recursive`):
- `normalize_key_order_suffix`: trailing ` (ASC)`/` (DESC)` → ` ASC`/` DESC`.
- direct: replace `$var` refs with their description (regex
  `\$[A-Za-z0-9_']+(?:\.[A-Za-z0-9_']+)*`).
- recursive: follow chains with cycle guard (`seen` set), replacing until stable;
  if a resolved description is itself a single var reference, follow it.
- **Regex**: this is the only regex in the pipeline. In `no_std` core, either
  (a) hand-write the matcher (the pattern is simple: `$` then a run of
  `[A-Za-z0-9_']` with optional `.`-separated segments), or (b) gate the
  resolve-vars feature behind `std` + `regex`. Recommend (a) hand-written scanner
  to keep resolution in core; it's ~30 lines. Two patterns: a "find all refs"
  scanner (replace) and a "whole-string is a single ref" check
  (`scalarVariableDescriptionRe`, anchored).

### 6.9 `reference.rs` — high-level entry points
Port `plantree/reference/reference.go`.
- `RenderMode { Auto, Plan, Profile }` (parse AUTO|PLAN|PROFILE).
- `Format { Traditional, Current, Compact }` (parse TRADITIONAL|CURRENT|COMPACT).
- `RenderConfig { wrap_width, hanging_indent, print_sections: Option<Vec<Section>>,
  show_scalar_vars, resolve_scalar_vars, resolve_scalar_vars_recursive }` (serde
  friendly for FFI/WASM).
- `render_tree_table_with_options(plan_nodes, mode, format, opts...)` and
  `..._with_config(...)`.

`render_tree_table` flow:
1. Validate non-empty plan_nodes, wrap_width >= 0.
2. `with_stats = match mode { Auto => has_stats(plan_nodes), Plan => false,
   Profile => true }`. `has_stats` = first node has `execution_stats` (see
   `queryplan.go` `HasStats`).
3. `rendered = process_tree(plan_nodes, format, opts)` →
   `QueryPlan::new` then `plantree::process_plan` with format-derived options:
   - `Traditional` → no extra opts.
   - `Current` → NodeTitle opts {KnownFlag=Label, ExecMethod=Angle, Target=On}.
   - `Compact` → Current opts + `EnableCompact`.
   - plus wrap_width / hanging_indent from opts.
4. `table_part = asciitable::render_table(rendered, spanner_table_spec(with_stats))`.
   Columns: ID(right)=format_id; Operator(left)=row.text(); if with_stats add
   Rows(right)=execution_stats.rows.total, Exec.(right)=execution_stats.execution_summary.num_executions,
   Total Latency(left)=execution_stats.latency.to_string().
5. `appendix_part = scalarappendix::render(rendered, print_options_from(opts))`.
6. return `table_part + appendix_part`.

Print presets/sections wrappers mirror `plantree/reference/appendix.go`
(`PrintSection`, `PrintPreset`, `NewPrintSections`, `ParsePrint*`).

### 6.10 Structured Plantree rows

`plantree_rows(plan_nodes, format, PlantreeConfig)` returns the `ProcessPlan`
pre-order rows without rendering a table or scalar appendix. Its narrow config
contains only `wrap_width`, `hanging_indent`, and `disallow_unknown_stats`.
WASM projects the core rows to the versioned v1 envelope described by
[`schema/plantree-rows-v1.schema.json`](schema/plantree-rows-v1.schema.json):
`{contractVersion: 1, rows}` or `{error}`. Each scalar child link carries raw
link fields plus an `isPredicate` classification from `QueryPlan::is_predicate`;
never infer that flag from `node_text` or formatted table output. Execution
statistics, render mode, occurrence IDs, and formatted `operator` text are
outside this contract.

---

## 7. Cross-cutting details & gotchas

1. **Ordering determinism** (§4.3): NodeTitle fields/labels SORTED; scalar-link
   groups FIRST-SEEN order. Metadata via `BTreeMap`.
2. **Nil-safety of getters**: Go's `proto.Get*()` returns zero on nil. Replicate
   with helper methods so ports are 1:1 and never panic on missing fields.
3. **`get_node_by_child_link(None)` → node[0]** (root). Preserve this (Go relies on
   `nil.GetChildIndex()==0`).
4. **Int-as-string in protojson**: `index`, `child_index` may be JSON strings.
5. **`DiscardUnknown`**: ignore unknown proto fields on decode (serde default,
   no `deny_unknown_fields`), but `--disallow-unknown-stats` DOES error on unknown
   keys inside `execution_stats` specifically.
6. **Wrap advance uses untrimmed chunk length** (§6.4). Off-by-one here corrupts
   wrapped output.
7. **Multi-line table cells**: table renderer must handle `\n` inside a cell
   (from wrapping) as stacked visual lines within one logical row.
8. **`format_id` `*` prefix** widens the ID column; alignment is display-width based.
9. **Latency formatting** is just `total + " " + unit` — no locale, no rounding
   (Spanner pre-formats, e.g. `"12.25"`, unit `"msecs"`).

---

## 8. Bindings

### 8.1 C ABI (`spannerplan-ffi`, cdylib, std)
Two entry points, both funneling into the same internal model and
`reference::render_tree_table_with_config` call — pick JSON for text-shaped
callers, wire bytes to avoid a wasted serialize/parse round trip when the caller
already holds protobuf bytes (see §5.1):
```c
// Wire (binary protobuf) — for callers holding raw bytes from a gRPC response
// or another language's protobuf library. Decoded via spannerplan-core's `wire`
// feature (prost-generated types, §5.3); no JSON involved.
char *spannerplan_render_tree_table_wire(
    const uint8_t *plan_wire, size_t plan_wire_len,
    const char *mode,        // "AUTO"|"PLAN"|"PROFILE"
    const char *format,      // "TRADITIONAL"|"CURRENT"|"COMPACT"
    const char *config_json, // RenderConfig JSON, or NULL for defaults
    int *out_is_error);

// JSON — for callers with a text-shaped plan (YAML/JSON file, REST response,
// JSON.stringify'd JS object from a browser/WASM host, scripting languages).
char *spannerplan_render_tree_table_json(
    const char *plan_json,   // JSON: QueryPlan | {planNodes:[...]} | ResultSet(Stats)
    const char *mode,
    const char *format,
    const char *config_json,
    int *out_is_error);

// Returns a NUL-terminated UTF-8 string that the caller must free with
// spannerplan_string_free. Returns NULL on allocation failure.
// On render error, *out_is_error is set to 1 and the string holds the message.
void spannerplan_string_free(char *s);
```
Implementation: decode (wire via `spannerplan-core`'s `wire` feature, or JSON via
the `spannerplan` std crate) into the shared internal model, call
`reference::render_tree_table_with_config`, marshal `Result` into the out-param +
`CString`. Catch panics at the boundary (`std::panic::catch_unwind`) and convert
to an error string. Provide a C header (`cbindgen`) and a `.def`/version script if
targeting stable symbols.

`config_json` stays JSON on both entry points — it's small, human-authored,
config-shaped data (equivalent to the CLI flags), not a hot path, so the
serialization cost is negligible even on the wire entry point. If a caller wants
to avoid JSON entirely for config too, expose builder-style setter functions as
an alternative later; not needed for v1.

Design note: keep the surface tiny so every language binds trivially
(Python/ctypes, Node/ffi-napi, Ruby/Fiddle, etc.). Richer typed FFI can come later.

### 8.2 WASM (`spannerplan-wasm`)
Mirror `examples/wasm/render`: export
`spannerplan_render_tree_table(plan_json_or_obj, mode, format, config)` returning
`{output}` or `{error}`. Use `wasm-bindgen` for the JS-friendly build, or a raw
`extern "C"` build reusing the FFI crate for a dependency-free `.wasm`.

Optionally also export a `Uint8Array`-accepting variant backed by the wire path
(§5.3/§8.1) for JS hosts that already have protobuf bytes (e.g. from a gRPC-Web
client) and want to skip a JSON round trip. Not required for v1 — add once the
JSON path is validated against the Go goldens.

---

## 9. Testing strategy — golden parity with Go

This is the backbone of correctness. Do NOT invent expected outputs; derive them
from the Go implementation.

1. **Copy fixtures** from the Go repo into `testdata/` (see
   [`testdata/README.md`](testdata/README.md)):
   - `reference/` ← `plantree/reference/testdata/{dca.yaml, distributed_cross_apply.yaml}`
   - `rendertree/` ← `cmd/rendertree/impl/testdata/*.yaml`
   - the `scalarAppendixPlanNodes()` synthetic plan from `reference_test.go`
     (reproduce as JSON when needed for unit tests).
2. **Generate golden outputs from Go** (done): `lab/genrsgolden` in the Go checkout
   renders the full mode × format × print × wrap × hanging-indent matrix into
   `testdata/golden/*.txt` (34 files). Regeneration steps:
   [`lab/genrsgolden/README.md`](lab/genrsgolden/README.md) — never edit goldens
   by hand. `reference_test.go` heredocs remain authoritative for spot checks.
3. **Rust golden tests**: for each fixture+options case, assert the Rust output
   equals the golden byte-for-byte. Store as `insta` snapshots or plain string
   compares.
4. **Unit tests** ported from Go table tests: `queryplan_test.go` (NodeTitle,
   link classification), `treerender_test.go` (wrapping edge cases, hanging
   indent), `asciitable_test.go` (table + appendix formatting, alignment,
   multi-line cells), `plantree_test.go`, `scalarappendix` behavior
   (`appendix_test.go`), `stats` extraction.
5. **`no_std` build gate in CI**: build `spannerplan-core` for a `no_std` target
   (e.g. `thumbv7em-none-eabi` or `--no-default-features` with a `no_std` shim
   test crate) to prevent accidental `std` leakage. Add `#![forbid(unsafe_code)]`
   to core if feasible.
6. **CJK/width test**: add one wide-character fixture and confirm alignment
   against a Go-produced golden (guards the `textwidth` port).
7. **Wire-decode parity**: for a subset of the golden fixtures, also produce a
   protobuf-wire-encoded copy (e.g. via a small Go helper using
   `proto.Marshal(&sppb.QueryPlan{...})`, or `protojson.Unmarshal` +
   `proto.Marshal` round-trip from the existing YAML/JSON fixtures) and assert
   the wire path (§5.3) produces the same internal model / same rendered output
   as the JSON path on the same underlying plan. This is the check that actually
   validates the `prost`+`protox` codegen and the `wire.rs` conversion layer,
   since golden-output comparisons alone don't exercise the wire decoder.

Suggested CI matrix: stable Rust, `cargo test --workspace`,
`cargo build -p spannerplan-core --no-default-features` (no_std proxy),
`cargo build -p spannerplan-core --target thumbv7em-none-eabi`,
`cargo build -p spannerplan-core --no-default-features --features wire --target
thumbv7em-none-eabi` (no_std with the wire codec enabled),
`cargo clippy --workspace -- -D warnings`, `cargo fmt --check`.

---

## 10. Dependencies (proposed)

Core (`spannerplan-core`), all `no_std`:
- `unicode-width` — display width.
- `unicode-segmentation` — grapheme clusters.
- `serde` (optional, `default-features=false`, `["derive","alloc"]`).
- `prost` + `prost-types` (optional, feature `wire`, `default-features=false` —
  drops `std`, keeps `alloc`). See §5.3.

Build-time only, for the `wire` feature's codegen (not a runtime dependency of
downstream consumers):
- `protox` — pure-Rust `.proto` → `FileDescriptorSet`, replaces `protoc`.
- `prost-build` — `FileDescriptorSet` → generated Rust, invoked from
  `spannerplan-core/build.rs` via `compile_fds`, with
  `.extern_path(".google.protobuf", "::prost_types")`.

std layer (`spannerplan`):
- `serde_json` — JSON decode.
- `serde_yaml` or `serde_yaml_ng` — YAML input (ports `protoyaml.YAMLToJSON`).
- `thiserror` (optional) for error ergonomics.

CLI (`spannerplan-cli`):
- `clap` (derive) for the flag surface (see §11), or hand-rolled to match Go's
  `flag` semantics precisely. Given deprecated aliases and mutual-exclusion rules,
  `clap` with manual post-validation is fine.

FFI: `libc` (optional), `cbindgen` (build/dev). WASM: `wasm-bindgen` (for JS build).

Avoid pulling `regex` into core (§6.8 uses a hand-written scanner).

---

## 11. CLI surface (`rendertree`)

Port the flags from `cmd/rendertree/impl/impl.go` (`run`):
`--mode` (AUTO), `--print` (basic), `--show-vars`, `--resolve-vars`,
`--resolve-vars-recursive`, `--disallow-unknown-stats`, `--execution-method`
(angle), `--target-metadata` (on), `--known-flag` (label), `--full-scan`
(deprecated alias → known-flag; mutually exclusive), `--compact`, `--inline-stats`,
`--wrap-width` (0), `--hanging-indent`, and the custom-column family
(`--custom`, `--custom-column`, `--custom-file` — defer, see non-goals).
Behavior: read stdin (YAML or JSON), `ExtractQueryPlan`, build `QueryPlan`, render.
Exit code 2 on usage errors (matches Go `usageError`).

---

## 12. Deferred features

Custom columns (`--custom` family), inline-stats, and direct struct interop with
a specific Spanner client crate remain out of scope for v1 (see non-goals in §1).
Parity is measured by Go-derived goldens, CLI diffs, wire-vs-JSON tests, and the
JS golden test across the mode × format × print × wrap matrix (§9).

---

## 13. Key Go source references (for cross-checking during the port)

- `queryplan.go`: `New`, `IsVisible`, `IsPredicate`, `IsFunction`, `GetLinkType`,
  `NodeTitle`, option types + `Parse*`.
- `treerender/treerender.go`: `Style`, `Row`, `renderTree`, `renderRow`,
  `wrapRowLines`, `wrapChunks`, `hangingIndentPadding`, `prefixLinesFromAncestor`,
  `PrefixMetrics`.
- `asciitable/asciitable.go`: `RenderTable`, `RenderAppendix`, `collectAppendixRows`.
- `stats/types.go`, `stats/extract.go`.
- `plantree/plantree.go`: `ProcessPlan`, `buildRenderedTree`, `RowWithPredicates`,
  `ScalarChildLink`.
- `internal/scalarappendix/appendix.go`: sections/presets parsing, `Render`,
  `scalarLinkLines`, resolver, `normalizeKeyOrderSuffix`, include predicates.
- `plantree/reference/reference.go` + `appendix.go`: high-level API.
- `cmd/rendertree/impl/impl.go`: CLI behavior + strings.
- `extract.go`, `protoyaml/protoyaml.go`: input detection + YAML/JSON handling.
- `plantree/reference/reference_test.go`: authoritative golden outputs.

---

## 14. Design decisions

1. **Serde in core vs. a bespoke decoder** — `serde` derive behind the optional
   `serde` feature (`spannerplan-core`). Keeps `no_std`; consumers pick their JSON
   stack in the std layer.
2. **Reimplement tablewriter vs. find a crate** — reimplemented in
   `asciitable.rs` (§6.7) for byte parity with Go `StyleASCII` output.
3. **regex for scalar-var resolution** — hand-written `$var` scanner in
   `scalarappendix.rs` (no `regex` in core).
4. **Custom columns** (Go `text/template`) — deferred (non-goals). Revisit with a
   minimal template or callback API if needed.
5. **Deprecated `RowWithPredicates` fields** (`Keys`, `ChildLinks`,
   `ContinuationIndent`) — dropped in Rust (no downstream yet).
6. **Wire codec choice** (§5.3) — `prost` + `protox` behind the `wire` feature;
   `wire.rs` converts generated types into `model.rs`. Hand-roll or `micropb`
   remain documented fallbacks in §5.3 if constraints change.
7. **Interop with existing Rust Spanner client crates** (§5.4) — deferred;
   evaluate against specific client crates when a consumer asks (field mapping
   may reuse `wire.rs` if types are prost-generated).
