# spannerplan (PHP)

PHP 8+ [FFI](https://www.php.net/manual/en/book.ffi.php) wrapper around
`libspannerplan_ffi` from `crates/spannerplan-ffi`.

## Requirements & caveats

- **Not a pure-PHP renderer** — the FFI extension loads the Rust cdylib.
- **PHP 8.0+** with the FFI extension enabled (`extension=ffi` in `php.ini`).
- **`ffi.enable=true` required** for CLI scripts (e.g.
  `php -d ffi.enable=true ...`); production `php.ini` may restrict FFI.
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
cd bindings/php
php -d ffi.enable=true test_render.php
```

The script renders `testdata/reference/dca.yaml` and checks for
`Distributed Cross Apply` in the output.

## `rendertree` CLI

```bash
cargo build -p spannerplan-ffi
export SPANNERPLAN_FFI_LIB="$PWD/target/debug/libspannerplan_ffi.dylib"
cd bindings/php
php -d ffi.enable=true bin/rendertree -mode plan < ../../testdata/reference/dca.yaml
```

Flags: `-mode`, `-format`, `-wrap-width`, `-help`. Requires `ffi.enable=true`
(see shebang comment in `bin/rendertree`). Exit `2` on flag/usage errors,
`1` on render failures.
