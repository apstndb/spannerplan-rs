# Golden output regeneration (`genrsgolden`)

Machine-generated expected outputs for byte-for-byte renderer parity live in
[`testdata/golden/`](../testdata/golden/). **Never edit those `.txt` files by
hand** — regenerate them from the Go reference implementation when fixtures or
render options change.

## Where the harness lives

The `genrsgolden` tool is maintained in the Go reference repo
[`github.com/apstndb/spannerplan`](https://github.com/apstndb/spannerplan) under
`lab/genrsgolden` (sibling to other `lab/` utilities such as `plan.jq`). Clone
that repo and run the harness from its root.

This Rust repo does not vendor the generator; it only stores the committed
golden outputs and the tests that consume them.

## Regeneration workflow

1. Check out a Go tree at the Spannerplan version you want to match (pin the
   same tag/commit CI uses for CLI parity, e.g. `v0.2.1`, when updating
   goldens for a release).
2. From the Go repo root, run `lab/genrsgolden` (exact invocation is defined in
   that tool's source — typically writes one file per matrix case).
3. Copy the generated `.txt` files into `testdata/golden/` in this repo,
   preserving filenames.
4. Run `cargo test -p spannerplan --test reference_golden` and
   `cd js && npm test` to confirm Rust and JS still match.

If the harness is unavailable, render each case with Go
`plantree/reference.RenderTreeTable*` using the same options as
[`reference_golden.rs`](../../crates/spannerplan/tests/reference_golden.rs).

## Golden filename matrix (34 files)

Pattern: `{prefix}_{mode}_{format}[_suffix].txt`

| Component | Values |
|-----------|--------|
| **prefix** | `dca` or `dcaplan` (see fixture table below) |
| **mode** | `auto`, `plan`, `profile` |
| **format** | `traditional`, `current`, `compact` |
| **wrap suffix** | `_wrap50`, `_wrap80`, `_wrap50_hanging`, `_wrap80_hanging` (plan + current only) |
| **print suffix** | `_print_enhanced`, `_print_full`, `_print_typed`, `_print_enhanced_showvars_resolverec` (plan + current only) |

Base cases (no suffix): `{prefix}_{mode}_{format}.txt` — 2 fixtures × 3 modes × 3
formats = 18 files. Wrap cases add 8 files (2 fixtures × 2 widths × 2 hanging
variants). Print-section cases add 8 files (2 fixtures × 4 print configs).

## Fixture prefix decoder ring

Both prefixes abbreviate "distributed cross apply", so the mapping is not
obvious from names alone:

| Golden prefix | Fixture file | Input shape | Execution stats in plan |
|---------------|--------------|-------------|-------------------------|
| `dca` | `reference/dca.yaml` | Full `ResultSet` (rows + `stats.queryPlan`) | **Yes** — per-node `executionStats` (profile fixture; root latency e.g. 12.25 msecs) |
| `dcaplan` | `reference/distributed_cross_apply.yaml` | `ResultSet` with `stats.queryPlan` only | **No** — plan structure only |

Rust tests map these in `reference_golden.rs` (`load_fixture`). The quick-start
example used in docs: `dca.yaml` → `dca_plan_current.txt`.

## Consumers

- Rust: `crates/spannerplan/tests/reference_golden.rs`
- JS: `js/packages/spannerplan/tests/golden.test.ts` (subset: `dca_plan_current.txt`)
- Wire: `crates/spannerplan/tests/wire_parity.rs` (subset of the same matrix)

See also `DESIGN.md` §9 and [`testdata/README.md`](../testdata/README.md).
