# Spanner client library examples

Run a query in **PLAN** or **PROFILE** mode with each language's official Spanner
client, take the returned `QueryPlan` protobuf (row data is discarded), and render
it with spannerplan — **without** round-tripping through JSON or YAML.

| Spanner `--query-mode` | Executes query? | PlanNode stats | spannerplan render mode |
|------------------------|-----------------|----------------|-------------------------|
| `PLAN` | No | Plan only | `PLAN` |
| `PROFILE` | Yes (rows discarded) | Plan + execution stats | `PROFILE` |

Pass options on the command line or via environment variables (flags take precedence).

| Flag | Environment | Description |
|------|-------------|-------------|
| `--query-mode` | `SPANNER_QUERY_MODE` | `PLAN` (default) or `PROFILE` |
| `--project` | `SPANNER_PROJECT_ID` | GCP project id |
| `--instance` | `SPANNER_INSTANCE_ID` | Spanner instance id |
| `--database` | `SPANNER_DATABASE_ID` | Database id |
| `--query` | `SPANNER_QUERY` | SQL text (overrides `--query-file`) |
| `--query-file` | `SPANNER_QUERY_FILE` | SQL file (default: [`query.sql`](query.sql)) |

## Prerequisites

- Built `spannerplan-ffi`: `cargo build -p spannerplan-ffi` from the repo root
- Google Cloud credentials with read access to your target database
  (`gcloud auth application-default login` is enough for many setups)
- Connection settings via flags or environment variables (**never commit `.env`**)

Optional: `GOOGLE_APPLICATION_CREDENTIALS` — service-account key file path.

Copy [`.env.example`](.env.example) to `.env` and fill in values locally.

Default SQL: [`query.sql`](query.sql) (override with `--query` or `--query-file`).

On JDK 24+, the Java example sets `MAVEN_OPTS=--sun-misc-unsafe-memory-access=allow`
via [`java/maven-opts.sh`](java/maven-opts.sh) to silence protobuf `sun.misc.Unsafe`
warnings from `exec:java` (which runs inside Maven's JVM).

`run-examples.sh` sets `SPANNERPLAN_QUIET_WASM_BUILD=1` for the Node build so
`wasm-pack` logs are suppressed (`--log-level error` + `cargo --quiet`).

Python/Java/Ruby/PHP dependencies are installed or built once before parallel runs to
avoid venv / Maven / Bundler / Composer races (e.g. concurrent installs into one
`.venv` or `vendor/bundle`).

## Examples

| Language | Directory | Renderer |
|----------|-----------|----------|
| Python | [`python/`](python/) | `bindings/python` FFI (`render_tree_table_wire`) |
| Java | [`java/`](java/) | `bindings/java` JNA |
| Node.js | [`node/`](node/) | `@spannerplan/core` WASM |
| Go | [`go/`](go/) | `spannerplan-ffi` via cgo |
| .NET | [`dotnet/`](dotnet/) | `bindings/dotnet` P/Invoke |
| Rust | [`rust/`](rust/) | `spannerplan` crate (`wire` + `render`) |
| C++ | [`cpp/`](cpp/) | `spannerplan-ffi` via `spannerplan.h` |
| Ruby | [`ruby/`](ruby/) | `bindings/ruby` Fiddle (`render_tree_table_wire`) |
| PHP | [`php/`](php/) | `bindings/php` FFI (`renderTreeTableWire`) |

## Quick run (Python)

```bash
export SPANNER_PROJECT_ID=...
export SPANNER_INSTANCE_ID=...
export SPANNER_DATABASE_ID=...
export SPANNERPLAN_FFI_LIB="$PWD/../../target/debug/libspannerplan_ffi.dylib"  # adjust platform

cd python
python3 -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
pip install -e ../../../bindings/python
python analyze_and_render.py
python analyze_and_render.py --query-mode PROFILE

# Or pass connection and SQL on the command line (no env vars required):
python analyze_and_render.py \
  --project my-project --instance my-instance --database my-db \
  --query 'SELECT * FROM Albums LIMIT 3'
```

Or run all examples (PLAN + PROFILE, in parallel) on this machine:

```bash
./run-examples.sh
```

### C++

Requires [google-cloud-cpp](https://github.com/googleapis/google-cloud-cpp) Spanner
(`find_package(google_cloud_cpp_spanner)`). The example uses a
[`cpp/vcpkg.json`](cpp/vcpkg.json) manifest (`google-cloud-cpp[spanner]` only) and
auto-detects vcpkg via [`vcpkg-detect.sh`](vcpkg-detect.sh).

**macOS / Linux — install vcpkg (one-time):**

```bash
git clone https://github.com/microsoft/vcpkg.git ~/vcpkg
~/vcpkg/bootstrap-vcpkg.sh
export VCPKG_ROOT=~/vcpkg   # add to your shell profile
```

**Build and run** (manifest mode installs `google-cloud-cpp[spanner]` on first
`cmake` configure; this can take a while):

```bash
cd cpp && ./run.sh --query-mode PROFILE
```

`./run.sh` and `run-examples.sh` pick up `VCPKG_ROOT`, `~/vcpkg`, or `vcpkg` on
`PATH`. Override with `CMAKE_TOOLCHAIN_FILE` if needed.

Without vcpkg, CMake may still succeed if `google-cloud-cpp` is installed
system-wide. Otherwise the C++ example is skipped in `run-examples.sh`.

### Ruby

Requires Ruby 3.0+ and Bundler matching `Gemfile.lock` (`BUNDLED WITH` — Homebrew
`bundle` is preferred over the macOS system stub). `run-examples.sh` resolves a
compatible `bundle` on `PATH` or under `/opt/homebrew/bin`.

```bash
cd ruby && ./run.sh
```

### PHP

Requires PHP 8+ with the `ffi` extension. The `grpc` extension is optional (REST
transport works without it); `composer install` uses
`--ignore-platform-req=ext-grpc` when needed.

```bash
cd php && ./run.sh
```

Each example prints an ASCII plan table to stdout on success.
