# testdata

Input fixtures and golden expected outputs for byte-for-byte parity with
[`apstndb/spannerplan`](https://github.com/apstndb/spannerplan) (Go). See
`DESIGN.md` §9 and `crates/spannerplan/tests/reference_golden.rs`.

## Layout

| Directory | Source (Go repo) | Purpose |
|-----------|------------------|---------|
| `reference/` | `plantree/reference/testdata/` | Primary YAML fixtures (`dca.yaml`, `distributed_cross_apply.yaml`) |
| `rendertree/` | `cmd/rendertree/impl/testdata/` | CLI parity fixtures (delete, profile variants, …) |
| `golden/` | Machine-generated from Go | Expected renderer output (34 `.txt` files) |
| `wire/` | Rust `gen_wire_fixtures` example | Protobuf `QueryPlan` bytes for wire-path tests |

Fixtures under `reference/` and `rendertree/` are copied verbatim from the Go
repo (Apache-2.0).

## Fixture ↔ golden prefix ↔ stats

Golden filenames use short prefixes. Both abbreviate "distributed cross apply":

| Golden prefix | Fixture file | Has per-node execution stats? | Notes |
|---------------|--------------|-------------------------------|-------|
| `dca` | `reference/dca.yaml` | **Yes** | Full `ResultSet` with result rows; `stats.queryPlan` includes `executionStats` (profile / latency columns) |
| `dcaplan` | `reference/distributed_cross_apply.yaml` | **No** | `stats.queryPlan` only; exercises plan tree without execution stats |

Example: `golden/dca_plan_current.txt` is `dca.yaml` rendered in plan mode with
the current format.

## Goldens

Goldens in `golden/` are **machine-generated**, not hand-transcribed. Regenerate
from a Go checkout with the `lab/genrsgolden` harness across the full
mode × format × print × wrap × hanging-indent matrix (34 files). Never edit
golden files by hand.

Full regeneration steps, filename matrix, and harness location:
[`lab/genrsgolden/README.md`](../lab/genrsgolden/README.md).

Quick start example used in docs and JS golden tests:
`reference/dca.yaml` → `golden/dca_plan_current.txt` (reference API) or
`golden/dca_rendertree_plan.txt` (`rendertree -mode plan` CLI).
