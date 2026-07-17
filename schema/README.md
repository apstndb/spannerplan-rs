# schema

Shared JSON Schema for cross-surface configuration. Today this is
`RenderConfig` — optional rendering options for the reference API
(`render_tree_table_with_config`).

| File | Role |
|------|------|
| [`render-config.schema.json`](render-config.schema.json) | JSON Schema (camelCase fields) |
| [`render-config.example.json`](render-config.example.json) | Example document |
| [`plantree-rows-v1.schema.json`](plantree-rows-v1.schema.json) | Versioned structured Plantree WASM response |

Consumers: Rust (`serde`), C FFI (`config_json`), WASM, `@spannerplan/core`,
and language bindings. Omitted fields use Rust defaults. See
[`ARCHITECTURE.md`](../ARCHITECTURE.md) (Reference API vs CLI path) and
[`DESIGN.md`](../DESIGN.md) §6.9 / §8.

`plantree-rows-v1.schema.json` instead describes output, not configuration.
It is the stable response union for `spannerplanPlantreeRows`: either
`{contractVersion: 1, rows}` or `{error}`. It intentionally contains raw row
structure rather than a formatted table, render mode, execution statistics, or
occurrence IDs.
