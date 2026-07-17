# Distribution

This project is distributed through **GitHub Releases** and **git dependencies**.
We do not publish to crates.io, npmjs.org, PyPI, Maven Central, NuGet, or
GitHub Packages. Official registries may be added later if discoverability
demands it.

Latest release: https://github.com/apstndb/spannerplan-rs/releases

Replace `v0.1.0-alpha.2` below with the tag you want.

## GitHub Releases

Tagged releases (`v*`) attach prebuilt artifacts:

| Asset | Consumer |
|-------|----------|
| `libspannerplan_ffi.{so,dylib,dll}` + `spannerplan.h` | FFI bindings (Python, Java, .NET, C++, Ruby, PHP) |
| `spannerplan-core-*.tgz` | `@spannerplan/core` (WASM-backed JS/TS library) |
| `spannerplan-cli-*.tgz` | `@spannerplan/cli` (`rendertree` npm binary) |
| `SHA256SUMS.txt` | Integrity verification |

Download everything for a tag:

```bash
gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs
```

Download only the FFI library for your OS (example: macOS arm64):

```bash
gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs \
  --pattern 'libspannerplan_ffi.dylib' --pattern 'spannerplan.h'
export SPANNERPLAN_FFI_DIR="$PWD"
```

## Rust (git dependency)

Library:

```toml
[dependencies]
spannerplan = { git = "https://github.com/apstndb/spannerplan-rs", tag = "v0.1.0-alpha.2" }
```

`no_std` core only:

```toml
[dependencies]
spannerplan-core = { git = "https://github.com/apstndb/spannerplan-rs", tag = "v0.1.0-alpha.2" }
```

CLI binary:

```bash
cargo install --git https://github.com/apstndb/spannerplan-rs --tag v0.1.0-alpha.2 spannerplan-cli
rendertree -mode plan < plan.yaml
```

Crates are marked `publish = false`; releases do not publish to crates.io.

## JavaScript / TypeScript

### From a release tarball (recommended — WASM prebuilt)

```bash
gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs --pattern 'spannerplan-core*.tgz'
npm install ./spannerplan-core-0.1.0-alpha.2.tgz
```

`package.json`:

```json
{
  "dependencies": {
    "@spannerplan/core": "file:./vendor/spannerplan-core-0.1.0-alpha.2.tgz"
  }
}
```

CLI from release:

```bash
gh release download v0.1.0-alpha.2 \
  --pattern 'spannerplan-core*.tgz' --pattern 'spannerplan-cli*.tgz'
npm install -g ./spannerplan-core-0.1.0-alpha.2.tgz ./spannerplan-cli-0.1.0-alpha.2.tgz
rendertree -mode plan < plan.yaml
```

Install both tarballs in the same `npm install` invocation. Do not install the
CLI tarball alone: `@spannerplan/core` is deliberately unpublished and cannot
be resolved from the npm registry.

### From git (builds WASM — requires Rust + wasm-pack)

```json
{
  "dependencies": {
    "@spannerplan/core": "github:apstndb/spannerplan-rs#v0.1.0-alpha.2&path:js/packages/spannerplan"
  }
}
```

Monorepo / submodule:

```json
{
  "dependencies": {
    "@spannerplan/core": "file:../spannerplan-rs/js/packages/spannerplan"
  }
}
```

Run `npm run build -w @spannerplan/core` (or `cd js && npm run build`) after
checkout.

## FFI bindings

Pattern for all FFI languages:

1. **Source** — clone or install the binding from git (`bindings/<lang>/`).
2. **Native library** — download `libspannerplan_ffi.*` (+ `spannerplan.h` for C++)
   from the [GitHub Release](https://github.com/apstndb/spannerplan-rs/releases).
3. **Point the binding** — `SPANNERPLAN_FFI_LIB` or `SPANNERPLAN_FFI_DIR`.

### Python

```bash
pip install "spannerplan @ git+https://github.com/apstndb/spannerplan-rs@v0.1.0-alpha.2#subdirectory=bindings/python"

gh release download v0.1.0-alpha.2 --pattern 'libspannerplan_ffi.*'
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.dylib"   # adjust extension
```

### Java

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs/bindings/java

gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs --pattern 'libspannerplan_ffi.so'
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.so"

mvn -q test
```

Add as a dependency via git submodule + local `mvn install`, or copy
`bindings/java` into your tree.

### .NET

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs

gh release download v0.1.0-alpha.2 --pattern 'spannerplan_ffi.dll'
export SPANNERPLAN_FFI_LIB="$PWD/spannerplan_ffi.dll"

dotnet test bindings/dotnet/SpannerPlan.sln
```

Reference `bindings/dotnet/src/SpannerPlan/SpannerPlan.csproj` from your
solution via project reference.

### Ruby

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs/bindings/ruby

gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs --pattern 'libspannerplan_ffi.dylib'
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.dylib"

gem build spannerplan.gemspec
gem install ./spannerplan-0.1.0.alpha.2.gem
```

### PHP

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs/bindings/php

gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs --pattern 'libspannerplan_ffi.so'
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.so"

composer install
php -d ffi.enable=true test_render.php
```

### C++

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs

gh release download v0.1.0-alpha.2 --pattern 'libspannerplan_ffi.*' --pattern 'spannerplan.h'
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.dylib"

cmake -S bindings/cpp -B bindings/cpp/build
cmake --build bindings/cpp/build
```

## Verification

After a release:

```bash
bash scripts/verify-release-consumers.sh v0.1.0-alpha.2
```

CI runs Rust git + Python git checks in the Release workflow (`verify-consumers` job).
