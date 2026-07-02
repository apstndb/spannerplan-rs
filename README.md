# spannerplan-rs

Render Cloud Spanner query plans as ASCII tables and appendices. Rust port of
[apstndb/spannerplan](https://github.com/apstndb/spannerplan) with byte-for-byte
parity against the Go reference on shared fixtures.

Use it as a **library** (Rust, JavaScript, Python, Java, .NET, and more) or as a
**`rendertree` CLI** (shell, Node, and several FFI sample binaries). Distribution
is via [GitHub Releases](https://github.com/apstndb/spannerplan-rs/releases) and
**git dependencies** — not crates.io or npmjs.org. See [`DISTRIBUTION.md`](DISTRIBUTION.md).

## Surfaces

| If you need… | Use |
|--------------|-----|
| Rust library / `no_std` embedding | `spannerplan` / `spannerplan-core` ([git dependency](DISTRIBUTION.md#rust-git-dependency)) |
| Shell / piping | `rendertree` ([Rust CLI](#rust-rendertree-cli) or [Go reference](https://github.com/apstndb/spannerplan)) |
| JavaScript / TypeScript (Node or browser) | [`@spannerplan/core`](js/packages/spannerplan) (WASM) |
| Python, Java, .NET, C++, Ruby, PHP | [`bindings/`](bindings/) over the FFI cdylib |

JavaScript uses WASM; FFI languages load a native `libspannerplan_ffi` from a
[release](https://github.com/apstndb/spannerplan-rs/releases) or a local build.
Caveats: [`bindings/README.md`](bindings/README.md#ffi-bindings-vs-native-implementations).

## Install

Replace `v0.1.0-alpha.1` with the [release tag](https://github.com/apstndb/spannerplan-rs/releases) you want.

**Rust** — `Cargo.toml`:

```toml
spannerplan = { git = "https://github.com/apstndb/spannerplan-rs", tag = "v0.1.0-alpha.1" }
```

**JavaScript** — prebuilt tarball from a release (WASM included):

```bash
gh release download v0.1.0-alpha.1 --repo apstndb/spannerplan-rs --pattern 'spannerplan-core*.tgz'
npm install ./spannerplan-core-0.1.0-alpha.1.tgz
```

**Python** — git + FFI library:

```bash
pip install "spannerplan @ git+https://github.com/apstndb/spannerplan-rs@v0.1.0-alpha.1#subdirectory=bindings/python"
export SPANNERPLAN_FFI_LIB=/path/to/libspannerplan_ffi.dylib   # from the release
```

More languages and detail: [`DISTRIBUTION.md`](DISTRIBUTION.md).

## Quick start

Sample input: [`testdata/reference/dca.yaml`](testdata/reference/dca.yaml).

### Rust library

```rust
use spannerplan::extract::extract_plan_nodes;
use spannerplan::core::reference::{render_tree_table, Format, RenderMode};

let yaml = std::fs::read_to_string("plan.yaml")?;
let nodes = extract_plan_nodes(yaml.as_bytes())?;
let table = render_tree_table(&nodes, RenderMode::Plan, Format::Current)?;
println!("{table}");
```

### Rust `rendertree` CLI

From a release:

```bash
cargo install --git https://github.com/apstndb/spannerplan-rs --tag v0.1.0-alpha.1 spannerplan-cli
rendertree -mode plan < plan.yaml
```

### Node.js

From a release tarball:

```bash
gh release download v0.1.0-alpha.1 --pattern 'spannerplan-cli*.tgz'
npm install -g ./spannerplan-cli-0.1.0-alpha.1.tgz
rendertree -mode plan < plan.yaml
```

In application code, `import { renderTreeTable } from "@spannerplan/core"`. See
[`js/packages/spannerplan/README.md`](js/packages/spannerplan/README.md).

### FFI bindings

Each binding under [`bindings/`](bindings/) has a README. Typical flow: install
from git, download `libspannerplan_ffi.*` from a release, set `SPANNERPLAN_FFI_LIB`.

```bash
# Python example
pip install "spannerplan @ git+https://github.com/apstndb/spannerplan-rs@v0.1.0-alpha.1#subdirectory=bindings/python"
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.so"
rendertree -mode plan < plan.yaml
```

## Examples

| Example | What it shows |
|---------|----------------|
| [`js/examples/rendertree-web`](js/examples/rendertree-web) | Browser UI: paste or upload YAML/JSON, render with `@spannerplan/core/browser` |
| [`js/packages/cli`](js/packages/cli) | Node `rendertree` binary (CLI parity with Go/Rust) |
| [`bindings/cpp`](bindings/cpp) | C++ `rendertree` and `render_example` linked against `spannerplan.h` |
| [`bindings/python`](bindings/python) | Python `rendertree` + library API (`render_tree_table_json`) |
| [`bindings/java`](bindings/java) | Java `Rendertree` main + JNA library |
| [`bindings/dotnet`](bindings/dotnet) | .NET `SpannerPlan.Cli` sample |
| [`bindings/ruby`](bindings/ruby) | Ruby `bin/rendertree` |
| [`bindings/php`](bindings/php) | PHP `bin/rendertree` (requires `ffi.enable`) |

**Browser demo** (`rendertree-web`):

```bash
cd js && npm install && npm run build -w @spannerplan/core
npm run dev -w rendertree-web
# open http://localhost:5173
```

Or use a release tarball for `@spannerplan/core` instead of building WASM locally
(see [`js/examples/rendertree-web/README.md`](js/examples/rendertree-web/README.md)).

Full `rendertree` command matrix by language:
[`bindings/README.md` — samples by language](bindings/README.md#rendertree-samples-by-language).

## Configuration

Shared render options (`wrapWidth`, `printSections`, scalar-var flags, …) are
described in [`schema/render-config.schema.json`](schema/render-config.schema.json)
with an example in [`schema/render-config.example.json`](schema/render-config.example.json).

## Further reading

| Document | Contents |
|----------|----------|
| [`DISTRIBUTION.md`](DISTRIBUTION.md) | Install from releases and git (all languages) |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Layers, WASM vs FFI, Go vs Rust/JS |
| [`bindings/README.md`](bindings/README.md) | FFI bindings and per-language READMEs |
| [`js/README.md`](js/README.md) | JavaScript workspace overview |
| [`DESIGN.md`](DESIGN.md) | Algorithm-level porting spec (maintainers) |

---

## For spannerplan-rs developers

### Repository layout

| Path | Contents |
|------|----------|
| `crates/spannerplan-core` | `no_std` renderer |
| `crates/spannerplan` | std YAML/JSON extract |
| `crates/spannerplan-cli` | `rendertree` binary |
| `crates/spannerplan-ffi` | C ABI (`cdylib`) |
| `crates/spannerplan-wasm` | `wasm-bindgen` exports |
| `js/` | `@spannerplan/core`, `@spannerplan/cli` |
| `bindings/` | FFI wrappers and sample CLIs |
| `testdata/` | Fixtures + Go-derived goldens ([`testdata/README.md`](testdata/README.md)) |
| `lab/genrsgolden/` | Regenerating goldens from Go ([`lab/genrsgolden/README.md`](lab/genrsgolden/README.md)) |

### Build and test

```bash
cargo test --workspace
cargo run -p spannerplan-cli -- -mode plan < testdata/reference/dca.yaml

cd js && npm install && npm run build && npm test
```

Go CLI parity tests in `spannerplan-cli` shell out to `rendertree` when on `PATH`
(skipped locally with a note if missing). CI sets `SPANNERPLAN_GO_PARITY=1`.
Install: `go install github.com/apstndb/spannerplan/cmd/rendertree@v0.1.11`.

### Build gates

```bash
cargo build -p spannerplan-core --no-default-features
cargo build -p spannerplan-core --target thumbv7em-none-eabi --no-default-features
cargo build -p spannerplan-core --target thumbv7em-none-eabi --no-default-features --features wire
```

CI: [`.github/workflows/ci.yml`](.github/workflows/ci.yml),
[`.github/workflows/bindings.yml`](.github/workflows/bindings.yml).

### Releases

Tag `v*` triggers [`.github/workflows/release.yml`](.github/workflows/release.yml)
(FFI artifacts + npm tarballs attached to GitHub Releases). Verify consumer
installs: `bash scripts/verify-release-consumers.sh v0.1.0-alpha.1`.

Rust crates are `publish = false`; releases do not publish to crates.io.
