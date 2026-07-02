# schema

Shared JSON Schema for cross-surface configuration. Today this is
`RenderConfig` — optional rendering options for the reference API
(`render_tree_table_with_config`).

| File | Role |
|------|------|
| [`render-config.schema.json`](render-config.schema.json) | JSON Schema (camelCase fields) |
| [`render-config.example.json`](render-config.example.json) | Example document |

Consumers: Rust (`serde`), C FFI (`config_json`), WASM, `@spannerplan/core`,
and language bindings. Omitted fields use Rust defaults. See
[`ARCHITECTURE.md`](../ARCHITECTURE.md) (Reference API vs CLI path) and
[`DESIGN.md`](../DESIGN.md) §6.9 / §8.
