# schema

Shared JSON Schema for cross-surface configuration. Today this is
`RenderConfig` — optional rendering options for the reference API
(`render_tree_table_with_config`).

| File | Role |
|------|------|
| [`render-config.schema.json`](render-config.schema.json) | JSON Schema (camelCase fields) |
| [`render-config.example.json`](render-config.example.json) | Example document |
| [`plantree-rows-v1alpha2.internal.schema.json`](plantree-rows-v1alpha2.internal.schema.json) | Bundled viewer Plantree response, wire revision 2 |

Consumers: Rust (`serde`), C FFI (`config_json`), WASM, `@spannerplan/core`,
and language bindings. Omitted fields use Rust defaults. See
[`ARCHITECTURE.md`](../ARCHITECTURE.md) (Reference API vs CLI path) and
[`DESIGN.md`](../DESIGN.md) §6.9 / §8.

`plantree-rows-v1alpha2.internal.schema.json` describes output, not
configuration. It is an internal contract between a checksum-pinned viewer and
its bundled renderer, not a stable external API. Success is
`{contractVersion: 2, rows}`; each row carries occurrence identity independent
of the Spanner `nodeId`.
