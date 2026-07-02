# Vendored protobuf subset

Apache-2.0 `.proto` files adapted from [googleapis/googleapis](https://github.com/googleapis/googleapis)
(`google/spanner/v1/query_plan.proto`, `result_set.proto`) and
[protocolbuffers/protobuf](https://github.com/protocolbuffers/protobuf)
(`google/protobuf/struct.proto`, used only at build time for protox parsing).

Runtime well-known types (`google.protobuf.Struct`/`Value`) come from
`prost-types` by default: `spannerplan-core/build.rs` filters
`google/protobuf/*` out of the vendored `FileDescriptorSet` so prost-build
maps those imports to `prost-types` without regenerating WKTs.
