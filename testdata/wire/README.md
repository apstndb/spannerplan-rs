# Wire-encoded plan fixtures

Protobuf `QueryPlan` wire bytes for JS (and other) wire-path parity tests.

| File | Source fixture |
|------|----------------|
| `dca_query_plan.bin` | `reference/dca.yaml` |
| `dcaplan_query_plan.bin` | `reference/distributed_cross_apply.yaml` |

Regenerate after model or proto changes:

```bash
cargo run -p spannerplan --example gen_wire_fixtures
```
