# Language bindings

FFI wrappers and samples around `crates/spannerplan-ffi` (C ABI) and
`crates/spannerplan-wasm` (browser/Node). Distribution: [`DISTRIBUTION.md`](../DISTRIBUTION.md)
(GitHub Releases + git; not crates.io/npm/PyPI). Full `rendertree` CLI parity lives in
Rust, Go, and `@spannerplan/cli`; other bindings expose the reference renderer
(JSON/YAML in, ASCII table out) unless noted.

Build the native library once from the repo root:

```bash
cargo build -p spannerplan-ffi
```

**Spanner client integration** (live `QueryPlan` → wire render, no JSON/YAML):
[`examples/spanner-client`](../examples/spanner-client). Set `SPANNER_*` env vars locally;
do not commit `.env`.

Set `SPANNERPLAN_FFI_LIB` to the platform cdylib when auto-detection fails
(`target/debug/libspannerplan_ffi.dylib`, `.so`, or `spannerplan_ffi.dll`).
All FFI bindings also honor `SPANNERPLAN_FFI_DIR` (directory containing the
platform library) and auto-detect `target/{debug,release}` plus downloaded
Release FFI artifacts under `artifacts/spannerplan-ffi-<platform>/`. See
per-language READMEs for resolution order.

### Native library resolution (FFI bindings)

| Order | Source |
|-------|--------|
| 1 | `SPANNERPLAN_FFI_LIB` — absolute path to the cdylib |
| 2 | `SPANNERPLAN_FFI_DIR` / platform library name |
| 3 | Monorepo `target/debug` or `target/release` |
| 4 | `artifacts/spannerplan-ffi-<platform>/` at repo root (CI downloads) |

## CI

The [Bindings](../.github/workflows/bindings.yml) workflow builds
`spannerplan-ffi` and runs binding test suites: Python (`pytest`), Java
(`mvn test`), .NET (`dotnet test`), Ruby, PHP, and C++ (`ctest`). The main
[CI](../.github/workflows/ci.yml) job still runs the ctypes smoke test in
`crates/spannerplan-ffi/tests/smoke.py`.

## FFI bindings vs native implementations

These packages are **thin wrappers** over the Rust `spannerplan-ffi` C ABI
(`cdylib`). They are not reimplementations of the renderer. You must ship or
locate a prebuilt native library per target OS/CPU.

For the layer cake, consumer matrix (Rust / Go / WASM / FFI), and Go vs
Rust/JS positioning, see [`ARCHITECTURE.md`](../ARCHITECTURE.md). This section
lists only the **FFI** language bindings:

| Binding | Mechanism | Notes |
|---------|-----------|-------|
| **Python** | `ctypes` | Install from git; bundle cdylib from [GitHub Releases](https://github.com/apstndb/spannerplan-rs/releases). |
| **Java** | JNA | Install from git; set `SPANNERPLAN_FFI_LIB` or `SPANNERPLAN_FFI_DIR`. |
| **.NET** | P/Invoke | Install from git / project reference; cdylib from GitHub Releases. |
| **C++** | Direct C ABI (`spannerplan.h`) | Link or load the cdylib at build/runtime. |
| **Ruby** | Fiddle | `spannerplan.gemspec` present; install from git + GitHub Release cdylib. Wire render via `render_tree_table_wire`. |
| **PHP** | `FFI` extension | `composer.json` present; install from git + cdylib; requires `ffi.enable=true`. Wire render via `renderTreeTableWire`. |

Rust (`spannerplan-core` / `spannerplan-cli`), Go
([`apstndb/spannerplan`](https://github.com/apstndb/spannerplan)), and
JavaScript (`@spannerplan/core` WASM) do not use this cdylib — see
[ARCHITECTURE.md — Single source of truth](../ARCHITECTURE.md#single-source-of-truth).

### Supported native platforms (FFI artifacts)

The [Release](../.github/workflows/release.yml) workflow publishes
[GitHub Releases](https://github.com/apstndb/spannerplan-rs/releases) with
`spannerplan-ffi` builds for:

| Artifact | Platform |
|----------|----------|
| `spannerplan-ffi-linux-x64` | Linux x86_64 (`libspannerplan_ffi.so`) |
| `spannerplan-ffi-macos-arm64` | macOS Apple Silicon (`libspannerplan_ffi.dylib`) |
| `spannerplan-ffi-macos-x64` | macOS Intel (`libspannerplan_ffi.dylib`) |
| `spannerplan-ffi-windows-x64` | Windows x64 (`spannerplan_ffi.dll`) |

Other architectures (Linux arm64, musl/Alpine, etc.) require building the
cdylib yourself with `cargo build -p spannerplan-ffi --release`.

### FFI boundary behavior (all native bindings)

- **Memory:** Render functions return a NUL-terminated UTF-8 string allocated
  in Rust; callers must free it with `spannerplan_string_free`. Bindings wrap
  this in RAII/finally-style cleanup — do not call the C API directly without
  freeing.
- **Errors:** On failure, `out_is_error` is set and the returned string holds
  the message (still must be freed). Bindings map this to exceptions or error
  types.
- **Panics:** Rust panics at the FFI boundary are caught and turned into error
  strings; they do not unwind across the C ABI.
- **Thread safety:** Treat concurrent calls as safe only when your binding
  loads a single shared library instance; each call is independent. No global
  mutable renderer state is exposed.

Layer cake and binding overview: [`ARCHITECTURE.md`](../ARCHITECTURE.md).

## `rendertree` samples by language

| Language | Path | How to run |
|----------|------|------------|
| **Rust** | `crates/spannerplan-cli` | `cargo run -p spannerplan-cli -- -mode plan < testdata/reference/dca.yaml` |
| **Go** | [apstndb/spannerplan `cmd/rendertree`](https://github.com/apstndb/spannerplan/tree/main/cmd/rendertree) | `go install github.com/apstndb/spannerplan/cmd/rendertree@v0.1.11` then `rendertree -mode plan < testdata/reference/dca.yaml` |
| **Node.js** | `js/packages/cli` | `cd js && npm install && npm run build && npx rendertree -mode plan < ../testdata/reference/dca.yaml` |
| **Browser** | `js/examples/rendertree-web` | `cd js && npm install && npm run build -w @spannerplan/core && npm run dev -w rendertree-web` |
| **C++** | `bindings/cpp` | `cmake -S bindings/cpp -B bindings/cpp/build && cmake --build bindings/cpp/build && bindings/cpp/build/rendertree -mode plan < testdata/reference/dca.yaml` |
| **Python** | `bindings/python` | `rendertree -mode plan < testdata/reference/dca.yaml` — see [python/README.md](python/README.md) |
| **Java** | `bindings/java` | `mvn -q exec:java -Dexec.mainClass=io.spannerplan.Rendertree -Dexec.args="-mode plan" < testdata/reference/dca.yaml` — see [java/README.md](java/README.md) |
| **.NET** | `bindings/dotnet` | `dotnet run --project src/SpannerPlan.Cli -mode plan < testdata/reference/dca.yaml` — see [dotnet/README.md](dotnet/README.md) |
| **Ruby** | `bindings/ruby` | `./bin/rendertree -mode plan < testdata/reference/dca.yaml` — see [ruby/README.md](ruby/README.md) |
| **PHP** | `bindings/php` | `php -d ffi.enable=true bin/rendertree -mode plan < testdata/reference/dca.yaml` — see [php/README.md](php/README.md) |

Rust, Go, and Node `rendertree` share CLI flags and table layout (including
profile-mode `Latency` column and appendix presets). C++ `rendertree` is a thin
stdin wrapper over the FFI reference renderer (`-mode`, `-format` only). Other
bindings call the same FFI entry points from application code.

## Per-language docs

- [C++](cpp/README.md)
- [.NET](dotnet/README.md)
- [Java](java/README.md)
- [PHP](php/README.md)
- [Python](python/README.md)
- [Ruby](ruby/README.md)

JavaScript packages: [`js/README.md`](../js/README.md).
