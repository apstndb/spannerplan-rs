# SpannerPlan (.NET)

P/Invoke wrapper around `libspannerplan_ffi` from `crates/spannerplan-ffi`.

## Requirements & caveats

- **Not a pure-.NET renderer** — P/Invoke loads the Rust `spannerplan-ffi`
  cdylib; rendering happens in native code.
- **Native library required** — `cargo build -p spannerplan-ffi` or a
  [Release FFI](../../.github/workflows/release-ffi.yml) artifact; NuGet
  distribution with bundled natives is not yet automated.
- **Platform-specific** — Linux x64, macOS arm64/x64, and Windows x64 in CI;
  set `SPANNERPLAN_FFI_LIB` or `SPANNERPLAN_FFI_DIR` when auto-detection fails.
- **FFI memory** — `spannerplan_string_free` is called after each render;
  Rust panics are caught and surfaced as `RenderError`.

See also: [bindings overview](../README.md#ffi-bindings-vs-native-implementations).

## Local development

Build the native library, then run tests:

```bash
cargo build -p spannerplan-ffi
export SPANNERPLAN_FFI_LIB="$PWD/target/debug/libspannerplan_ffi.dylib"  # or .so on Linux
cd bindings/dotnet
dotnet test
```

When `SPANNERPLAN_FFI_LIB` is unset, the library walks up from the process base directory looking for `target/debug/` under the monorepo root.

## Usage

```csharp
using SpannerPlan;

var table = PlanRenderer.RenderTreeTableJson(planYaml, mode: "PLAN", format: "CURRENT");
```

## `rendertree` CLI

Sample console app that reads plan YAML or JSON from stdin and prints the ASCII
table to stdout:

```bash
cargo build -p spannerplan-ffi
export SPANNERPLAN_FFI_LIB="$PWD/target/debug/libspannerplan_ffi.dylib"
cd bindings/dotnet
dotnet run --project src/SpannerPlan.Cli -mode plan < ../../testdata/reference/dca.yaml
```

Flags: `-mode`, `-format`, `-wrap-width`, `-help`. Exit `2` on flag/usage errors,
`1` on render failures.

NuGet distribution requires prebuilt native libraries per platform (not yet automated).

## API

| Method | Description |
|--------|-------------|
| `RenderTreeTableJson` | Render from JSON/YAML text (QueryPlan, ResultSetStats, or ResultSet shapes) |
| `RenderTreeTableWire` | Render from protobuf wire-encoded plan bytes |

Both accept optional `RenderConfig` (serialized to JSON for the FFI `config_json` argument). On render failure, `RenderError` is thrown with the native error message.
