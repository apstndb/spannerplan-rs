# Structured Plantree golden generator

This self-contained Go module creates the bundled viewer's v1alpha2 Plantree
goldens (numeric wire revision 2) in [`testdata/golden/`](../../testdata/golden/). It is pinned to
`github.com/apstndb/spannerplan v0.3.0-alpha.1`, the same reference version as
the release parity gate. The source YAML fixtures remain verbatim captures
copied from the earlier provenance-pinned tag.

The tool reads the real `dca.yaml` and `distributed_cross_apply.yaml` fixtures,
calls Go `plantree.ProcessPlan` with the reference `CURRENT` formatting options,
and writes the internal occurrence-preserving projection. It classifies each
scalar child link with `QueryPlan.IsPredicate`; it does not infer structure
from formatted operator text.

From this directory:

```bash
go run . -repo-root ../..
go run . -repo-root ../.. -check
```

The second command must reproduce the committed JSON byte-for-byte. Goldens are
pretty-printed with a trailing newline and always encode empty arrays as `[]`.
Never hand-edit the generated JSON.
