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

Release FFI assets are versioned target-triple archives, not loose files.
Extract the matching `spannerplan-ffi-<version>-<target-triple>.tar.gz` (or
Windows `.zip`) and point `SPANNERPLAN_FFI_LIB`/`SPANNERPLAN_FFI_DIR` at its
contents; the archive also includes `spannerplan.h` and `LICENSE`.

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
2. `SPANNERPLAN_FFI_DIR` — directory containing the platform library (extracted release archive or local staging)
3. Bundled wheel layout: `spannerplan/lib/<lib>` next to the package
4. Monorepo checkout: `target/debug` or `target/release` at the repo root
5. Legacy CI artifact directories under the repo root (compatibility only)

## GitHub Releases

The [Release](../../.github/workflows/release.yml) workflow attaches versioned
`spannerplan-ffi` target-triple archives for Linux x64, macOS arm64/x64, and Windows x64 to
each [GitHub Release](https://github.com/apstndb/spannerplan-rs/releases):

| File in release | Platform |
|---|---|
| `spannerplan-ffi-<version>-x86_64-unknown-linux-gnu.tar.gz` | Linux x64 |
| `spannerplan-ffi-<version>-aarch64-apple-darwin.tar.gz` | macOS arm64 |
| `spannerplan-ffi-<version>-x86_64-apple-darwin.tar.gz` | macOS x64 |
| `spannerplan-ffi-<version>-x86_64-pc-windows-msvc.zip` | Windows x64 |

Download for your platform:

```bash
gh release download v0.1.0-alpha.3 --repo apstndb/spannerplan-rs \
  --pattern 'spannerplan-ffi-0.1.0-alpha.3-aarch64-apple-darwin.tar.gz'
tar -xzf spannerplan-ffi-0.1.0-alpha.3-aarch64-apple-darwin.tar.gz
export SPANNERPLAN_FFI_DIR="$PWD"
cd bindings/python && pytest
```

See also [`DISTRIBUTION.md`](../../DISTRIBUTION.md).
