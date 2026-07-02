# spannerplan (Python)

ctypes wrapper around `libspannerplan_ffi` from `crates/spannerplan-ffi`.

## Requirements & caveats

- **Not a pure-Python renderer** — all rendering runs in the Rust cdylib; this
  package only loads it via `ctypes`.
- **Native library required** — build with `cargo build -p spannerplan-ffi` or
  download a [Release FFI](../../.github/workflows/release-ffi.yml) artifact.
- **Platform-specific** — ship the matching `.so` / `.dylib` / `.dll` per
  OS/CPU (Linux x64, macOS arm64/x64, Windows x64 in CI).
- **PyPI wheels** — layout is documented below but wheel publishing is not yet
  automated in this repo; use `pip install -e .` locally or bundle the cdylib.
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

## CI artifacts

The [Release FFI](../../.github/workflows/release-ffi.yml) workflow builds
`spannerplan-ffi` cdylibs for Linux x64, macOS arm64/x64, and Windows x64 and
uploads them as GitHub Actions artifacts:

| Artifact | Platform |
|---|---|
| `spannerplan-ffi-linux-x64` | `libspannerplan_ffi.so` |
| `spannerplan-ffi-macos-arm64` | `libspannerplan_ffi.dylib` |
| `spannerplan-ffi-macos-x64` | `libspannerplan_ffi.dylib` |
| `spannerplan-ffi-windows-x64` | `spannerplan_ffi.dll` |

Download artifacts for the current platform and point the bindings at the directory:

```bash
gh run download <run-id> -D artifacts
export SPANNERPLAN_FFI_DIR="$PWD/artifacts/spannerplan-ffi-linux-x64"  # adjust per OS
cd bindings/python && pytest
```

Alternatively, after downloading into the default artifact layout at the repo
root, the package auto-detects `artifacts/spannerplan-ffi-<platform>/`.

## Wheels and manylinux

PyPI distribution ships platform wheels that bundle the prebuilt cdylib. The
Python code is pure `ctypes`; only the native library is platform-specific.

**Linux (manylinux):** build the cdylib on a manylinux image (or via the Release
FFI workflow on `ubuntu-latest`), then audit with
[`auditwheel`](https://github.com/pypa/auditwheel) before bundling into the
wheel:

```bash
auditwheel repair dist/spannerplan-*.whl
```

Target `manylinux_2_28_x86_64` (or newer) for broad pip compatibility on
glibc-based Linux. musl/Alpine wheels are not produced by the current matrix.

**macOS:** ship separate `macosx_11_0_arm64` and `macosx_10_9_x86_64` wheels (or
a single arch per wheel). Use the Release FFI macOS artifacts as inputs.

**Windows:** bundle `spannerplan_ffi.dll` from the Windows x64 artifact into a
`win_amd64` wheel.

Wheel layout (not yet automated in this repo):

```
src/spannerplan/
  __init__.py
  lib/
    libspannerplan_ffi.so   # platform-specific name per build
```

`pyproject.toml` includes optional platform classifiers for when wheels are
published; see `[project.classifiers]`.
