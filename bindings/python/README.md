# spannerplan (Python)

ctypes wrapper around `libspannerplan_ffi` from `crates/spannerplan-ffi`.

## Requirements & caveats

- **Not a pure-Python renderer** — all rendering runs in the Rust cdylib; this
  package only loads it via `ctypes`.
- **Native library required** — build with `cargo build -p spannerplan-ffi` or
  download a [GitHub Release](https://github.com/apstndb/spannerplan-rs/releases) FFI artifact.
- **Install from git** — `pip install "spannerplan @ git+https://github.com/apstndb/spannerplan-rs@TAG#subdirectory=bindings/python"`;
  see [`DISTRIBUTION.md`](../../DISTRIBUTION.md).
- **FFI memory** — returned strings are freed via `spannerplan_string_free` in
  the binding; panics in Rust are caught and surfaced as `RenderError`.

See also: [bindings overview](../README.md#ffi-bindings-vs-native-implementations).

## Local development

Build the native library, then run tests:

```bash
cargo build -p spannerplan-ffi
export SPANNERPLAN_FFI_LIB="$PWD/target/debug/libspannerplan_ffi.dylib"  # or .so / .dll
cd bindings/python
pip install -e ".[dev]"
pytest
```

Render a plan from stdin:

```bash
rendertree < ../../testdata/reference/dca.yaml
rendertree -mode plan < ../../testdata/reference/dca.yaml
```

Flags: `-mode`, `-print`, `-compact`, `-wrap-width`, `-h` (usage errors exit 2).

The package resolves the native library in this order:

1. `SPANNERPLAN_FFI_LIB` — absolute path to the cdylib
2. `SPANNERPLAN_FFI_DIR` — directory containing the platform library (CI artifacts, local staging)
3. Bundled wheel layout: `spannerplan/lib/<lib>` next to the package
4. Monorepo checkout: `target/debug` or `target/release` at the repo root
5. CI artifact directories under the repo root (see below)

## GitHub Releases

The [Release](../../.github/workflows/release.yml) workflow attaches
`spannerplan-ffi` cdylibs for Linux x64, macOS arm64/x64, and Windows x64 to
each [GitHub Release](https://github.com/apstndb/spannerplan-rs/releases):

| File in release | Platform |
|---|---|
| `libspannerplan_ffi.so` | Linux x64 |
| `libspannerplan_ffi.dylib` | macOS (arm64 and x64 builds share the extension) |
| `spannerplan_ffi.dll` | Windows x64 |

Download for your platform:

```bash
gh release download v0.1.0-alpha.1 --repo apstndb/spannerplan-rs \
  --pattern 'libspannerplan_ffi.dylib'   # or .so / .dll
export SPANNERPLAN_FFI_DIR="$PWD"
cd bindings/python && pytest
```

See also [`DISTRIBUTION.md`](../../DISTRIBUTION.md).
