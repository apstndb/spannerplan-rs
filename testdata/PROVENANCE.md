# testdata provenance

Records where each copied fixture tree came from, the upstream ref used, and how
to regenerate it. See [`README.md`](README.md) for the layout and the fixture ↔
golden mapping.

Upstream reference implementation:
[`github.com/apstndb/spannerplan`](https://github.com/apstndb/spannerplan)
(Apache-2.0). The pinned version tracks the Go CLI parity pin in
[`.github/workflows/ci.yml`](../.github/workflows/ci.yml).

| Directory | Origin | Upstream path | Ref / version | Last synced |
|-----------|--------|---------------|---------------|-------------|
| `reference/` | Copied verbatim from Go repo | `plantree/reference/testdata/` | `v0.2.0` | 2026-07-08 |
| `rendertree/` | Copied verbatim from Go repo | `cmd/rendertree/impl/testdata/` | `v0.2.0` | 2026-07-08 |
| `golden/` | Machine-generated from Go repo | rendered via `lab/genrsgolden` | `v0.2.0` | 2026-07-08 |
| `wire/` | Generated in this repo (Rust) | derived from `reference/*.yaml` | this repo | 2026-07-08 |

The `v0.2.0` fixture copies under `reference/` and `rendertree/` were verified
byte-identical to the upstream tree at that tag on the last-synced date.

## Regenerating each tree

### `reference/` and `rendertree/` (verbatim copies)

```bash
git clone --depth 1 --branch v0.2.0 https://github.com/apstndb/spannerplan.git
cp spannerplan/plantree/reference/testdata/*.yaml       testdata/reference/
cp spannerplan/cmd/rendertree/impl/testdata/*.yaml      testdata/rendertree/
rm -rf spannerplan
```

### `golden/` (machine-generated expected output)

Regenerate from a Go checkout at the same tag using the `lab/genrsgolden`
harness. Never edit the `.txt` files by hand. Full steps and the filename
matrix: [`lab/genrsgolden/README.md`](../lab/genrsgolden/README.md).

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
