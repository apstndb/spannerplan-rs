# testdata provenance

Records where each copied fixture tree came from, the upstream ref used, and how
to regenerate it. See [`README.md`](README.md) for the layout and the fixture ↔
golden mapping.

Upstream reference implementation:
[`github.com/apstndb/spannerplan`](https://github.com/apstndb/spannerplan)
(Apache-2.0). Generated projections record their Go parity version; verbatim
fixture copies retain the tag from which their source bytes were copied.

| Directory | Origin | Upstream path | Ref / version | Last synced |
|-----------|--------|---------------|---------------|-------------|
| `reference/` | Copied verbatim from Go repo | `plantree/reference/testdata/` | `v0.2.1` | 2026-07-08 |
| `rendertree/` | Copied verbatim from Go repo | `cmd/rendertree/impl/testdata/` | `v0.2.1` | 2026-07-08 |
| `golden/` | Machine-generated from Go repo | rendered via `lab/genrsgolden` | `v0.2.1` | 2026-07-08 |
| `golden/*_plantree_rows_current.json` | Machine-generated from Go repo | projected via `lab/genstructuredgolden` | `v0.3.0-alpha.2` | 2026-07-18 |
| `golden/dca.signature.txt` | Copied verbatim from Go repo | `plantree/testdata/signature/dca.signature.txt` | `v0.3.0-alpha.2`, peeled tag commit `a92d753cbff4ea95f5ba5b372b44ce50038dc15b` | 2026-07-18 |
| `wire/` | Generated in this repo (Rust) | derived from `reference/*.yaml` | this repo | 2026-07-08 |

The `v0.2.1` fixture copies under `reference/` and `rendertree/` were verified
byte-identical to the upstream tree at that tag on the last-synced date.

## Regenerating each tree

### `reference/` and `rendertree/` (verbatim copies)

```bash
git clone --depth 1 --branch v0.2.1 https://github.com/apstndb/spannerplan.git
cp spannerplan/plantree/reference/testdata/*.yaml       testdata/reference/
cp spannerplan/cmd/rendertree/impl/testdata/*.yaml      testdata/rendertree/
rm -rf spannerplan
```

### `golden/` (machine-generated expected output)

Regenerate from a Go checkout at the same tag using the `lab/genrsgolden`
harness. Never edit the `.txt` files by hand. Full steps and the filename
matrix: [`lab/genrsgolden/README.md`](../lab/genrsgolden/README.md).

### Internal structured Plantree v1alpha2 JSON goldens

The structured JSON is generated locally from the pinned Go v0.3.0-alpha.2 module,
not hand-authored and not derived by parsing ASCII tables:

```bash
cd lab/genstructuredgolden
go run . -repo-root ../..
go run . -repo-root ../.. -check
```

The generator uses Go `plantree.ProcessPlan` with the reference `CURRENT`
options and `QueryPlan.IsPredicate` for scalar-link classification. It writes
the two `*_plantree_rows_current.json` files with deterministic indentation,
a trailing newline, and `[]` for empty slices.

### Structural signature golden (verbatim copy)

`golden/dca.signature.txt` is a Go-owned alpha artifact. It was copied
byte-for-byte from `apstndb/spannerplan` commit
`a92d753cbff4ea95f5ba5b372b44ce50038dc15b` (the peeled `v0.3.0-alpha.2` tag), with SHA-256
`d0db9898f99e6313ef782d6e8e672db07c1367ac639380064217f01622dfa974`.
Replace it only by re-copying the corresponding upstream file from a newer
explicit Go pin and updating this provenance record; never regenerate it from
the Rust implementation.

### `wire/` (protobuf `QueryPlan` bytes)

Generated in this repository from the `reference/` YAML fixtures, not copied from
Go. Regenerate after model or proto changes:

```bash
cargo run -p spannerplan --example gen_wire_fixtures
```

See [`wire/README.md`](wire/README.md) for the file ↔ fixture mapping.

## Updating this file

When bumping the Go parity pin or resyncing fixtures, update the `Ref / version`
and `Last synced` columns above (and the pin in `ci.yml`). Use the upstream
version tag rather than a raw commit SHA unless a specific unreleased commit is
required.
