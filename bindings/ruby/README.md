# spannerplan (Ruby)

Fiddle wrapper around `libspannerplan_ffi` from `crates/spannerplan-ffi`.

## Requirements & caveats

- **Not a pure-Ruby renderer** — Fiddle loads the Rust cdylib; gem
  distribution with bundled natives is not yet automated.
- **Native library required** — `cargo build -p spannerplan-ffi` or a
  [Release FFI](../../.github/workflows/release-ffi.yml) artifact; set
  `SPANNERPLAN_FFI_LIB` or `SPANNERPLAN_FFI_DIR` when auto-detection fails.
- **Platform-specific** — Linux x64, macOS arm64/x64, Windows x64 in CI.
- **FFI memory** — `spannerplan_string_free` is called after each render.

See also: [bindings overview](../README.md#ffi-bindings-vs-native-implementations).

## Local development

Build the native library, then run the test script:

```bash
cargo build -p spannerplan-ffi
export SPANNERPLAN_FFI_LIB="$PWD/target/debug/libspannerplan_ffi.dylib"  # or .so on Linux
cd bindings/ruby
ruby test/test_render.rb
```

The test renders `testdata/reference/dca.yaml` and checks for
`Distributed Cross Apply` in the output.

## `rendertree` CLI

```bash
cargo build -p spannerplan-ffi
export SPANNERPLAN_FFI_LIB="$PWD/target/debug/libspannerplan_ffi.dylib"
cd bindings/ruby
chmod +x bin/rendertree
./bin/rendertree -mode plan < ../../testdata/reference/dca.yaml
```

Flags: `-mode`, `-format`, `-wrap-width`, `-help`. Exit `2` on flag/usage errors,
`1` on render failures.
