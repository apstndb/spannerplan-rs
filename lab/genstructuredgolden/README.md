# Structured Plantree golden generator

This self-contained Go module creates the version-1 structured Plantree
goldens in [`testdata/golden/`](../../testdata/golden/). It is pinned to
`github.com/apstndb/spannerplan v0.2.1`, the same reference version as the
copied fixtures.

The tool reads the real `dca.yaml` and `distributed_cross_apply.yaml` fixtures,
calls Go `plantree.ProcessPlan` with the reference `CURRENT` formatting options,
and writes only the v1 projection. It classifies each scalar child link with
`QueryPlan.IsPredicate`; it does not inspect formatted operator text.

From this directory:

```bash
go run . -repo-root ../..
go run . -repo-root ../.. -check
```

The second command must reproduce the committed JSON byte-for-byte. Goldens are
pretty-printed with a trailing newline and always encode empty arrays as `[]`.
Never hand-edit the generated JSON.
