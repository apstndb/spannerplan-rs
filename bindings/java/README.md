# spannerplan (Java)

[JNA](https://github.com/java-native-access/jna) wrapper around `libspannerplan_ffi` from `crates/spannerplan-ffi`.

## Requirements & caveats

- **Not a pure-Java renderer** — JNA loads the Rust `spannerplan-ffi` cdylib;
  no rendering logic runs in the JVM.
- **Native library required** — `cargo build -p spannerplan-ffi` or a
  [GitHub Release](https://github.com/apstndb/spannerplan-rs/releases) FFI artifact;
  the JAR does not bundle the cdylib. See [`DISTRIBUTION.md`](../../DISTRIBUTION.md).
- **Platform-specific** — provide the matching library for Linux x64, macOS
  arm64/x64, or Windows x64 (CI matrix); other targets need a local build.
- **JDK 17+**, **Maven 3.9+**.
- **FFI memory** — JNA frees render outputs via `spannerplan_string_free`;
  Rust panics are caught at the boundary and returned as error strings.

Release FFI assets are versioned target-triple archives rather than loose
libraries. Extract the matching archive before setting `SPANNERPLAN_FFI_LIB`
or `SPANNERPLAN_FFI_DIR`; archives include the natural library filename,
`spannerplan.h`, and `LICENSE`.

See also: [bindings overview](../README.md#ffi-bindings-vs-native-implementations).

## Local development

Build the native library from the repository root, then run tests:

```bash
cargo build -p spannerplan-ffi
export SPANNERPLAN_FFI_LIB="$PWD/target/debug/libspannerplan_ffi.dylib"  # or .so on Linux
cd bindings/java
mvn test
```

If `SPANNERPLAN_FFI_LIB` is unset, the binding looks for `SPANNERPLAN_FFI_DIR`,
then `target/{debug,release}/libspannerplan_ffi.{dylib,so,dll}` under the
monorepo root, then extracted Release FFI archives.

## API

```java
import io.spannerplan.Spannerplan;

String table = Spannerplan.renderTreeTableJson(yamlOrJsonText);
String tableWire = Spannerplan.renderTreeTableWire(protobufBytes);
```

Optional `mode`, `format`, and `config` map overloads mirror the Python binding.

Errors from the renderer raise `io.spannerplan.RenderError`.

## `rendertree` CLI

Render a plan from stdin:

```bash
mvn -q exec:java -Dexec.mainClass=io.spannerplan.Rendertree < ../../testdata/reference/dca.yaml
mvn -q exec:java -Dexec.mainClass=io.spannerplan.Rendertree -Dexec.args="-mode plan" < ../../testdata/reference/dca.yaml
```

Build a runnable jar (requires `mvn package` with the shade plugin):

```bash
mvn -q package
java -jar target/spannerplan-java-0.1.0-shaded.jar -mode plan < ../../testdata/reference/dca.yaml
```

Flags: `-mode`, `-print`, `-compact`, `-wrap-width`, `-h` (usage errors exit 2).

## Install locally

```bash
mvn install
```

Add the artifact `io.spannerplan:spannerplan-java:0.1.0` to your project after installing to the local Maven repository.
