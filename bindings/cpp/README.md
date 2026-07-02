# spannerplan (C++)

CMake examples linking `spannerplan.h` and `libspannerplan_ffi`.

## Requirements & caveats

- **Thin C++ wrapper over Rust FFI** — links or loads `libspannerplan_ffi`;
  no C++ rendering implementation.
- **Native library required** — build with `cargo build -p spannerplan-ffi`
  or use a [Release FFI](../../.github/workflows/release-ffi.yml) artifact.
- **Platform-specific** — CI builds Linux x64, macOS arm64/x64, Windows x64;
  pass `-DSPANNERPLAN_FFI_LIB=...` or set the env var when CMake cannot find
  `target/debug/`.
- **FFI memory** — call `spannerplan_string_free` on every non-NULL return
  from render entry points (see `spannerplan.h`).

See also: [bindings overview](../README.md#ffi-bindings-vs-native-implementations).

## Local development

Build the native library, then compile the tools:

```bash
cargo build -p spannerplan-ffi
cd bindings/cpp
cmake -S . -B build
cmake --build build
```

### `rendertree`

Reads YAML or JSON from stdin and prints the rendered table (reference renderer
via FFI; supports `-mode` and `-format`):

```bash
./build/rendertree -mode plan < ../../testdata/reference/dca.yaml
./build/rendertree -mode profile -format current < ../../testdata/reference/dca.yaml
```

### `render_example` (optional)

The file-path sample is built by default. Disable with
`-DSPANNERPLAN_BUILD_RENDER_EXAMPLE=OFF`:

```bash
./build/render_example ../../testdata/reference/dca.yaml
```

Override the library path when needed:

```bash
export SPANNERPLAN_FFI_LIB="$PWD/target/debug/libspannerplan_ffi.dylib"  # or .so on Linux
# or: export SPANNERPLAN_FFI_DIR="$PWD/artifacts/spannerplan-ffi-macos-arm64"
cd bindings/cpp
cmake -S . -B build -DSPANNERPLAN_FFI_LIB="$SPANNERPLAN_FFI_LIB"
ctest --test-dir build --output-on-failure
```
