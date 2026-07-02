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

Python/Java dependencies are installed or built once before parallel runs to
avoid venv / Maven races (e.g. concurrent `pip install -e` into one `.venv`).

## Examples

| Language | Directory | Renderer |
|----------|-----------|----------|
| Python | [`python/`](python/) | `bindings/python` FFI (`render_tree_table_wire`) |
| Java | [`java/`](java/) | `bindings/java` JNA |
| Node.js | [`node/`](node/) | `@spannerplan/core` WASM |
| Go | [`go/`](go/) | `spannerplan-ffi` via cgo |
| .NET | [`dotnet/`](dotnet/) | `bindings/dotnet` P/Invoke |
| Rust | [`rust/`](rust/) | `spannerplan` crate (`wire` + `render`) |

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

Each example prints an ASCII plan table to stdout on success.
