# Distribution

Alpha releases are distributed through **GitHub Releases** and **git
dependencies**. The beta milestone will add crates.io and npmjs.org publication
for the supported public Rust and JavaScript packages after their APIs are
stabilized. PyPI, Maven Central, NuGet, and GitHub Packages are not currently
part of that milestone.

Latest release: https://github.com/apstndb/spannerplan-rs/releases

Replace `v0.1.0-alpha.2` below with the tag you want.

## GitHub Releases

Tagged releases (`v*`) attach prebuilt artifacts:

| Asset | Consumer |
|-------|----------|
| `spannerplan-ffi-${VERSION}-{target}.tar.gz` or `.zip` | FFI bindings (Python, Java, .NET, C++, Ruby, PHP); each archive contains the natural library filename, `spannerplan.h`, and `LICENSE` |
| `spannerplan-core-*.tgz` | `@spannerplan/core` (WASM-backed JS/TS library) |
| `spannerplan-cli-*.tgz` | `@spannerplan/cli` (`rendertree` npm binary) |
| `SHA256SUMS.txt` | Integrity verification |

The versioned target-triple archives supersede the alpha.1 loose native/header
layout. Do not download or configure a loose `libspannerplan_ffi.*` asset from
an older release when an archive is available.

Download everything for a tag:

```bash
gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs
```

Download and extract the FFI archive for your OS (example: macOS arm64):

```bash
gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs \
  --pattern 'spannerplan-ffi-*-aarch64-apple-darwin.tar.gz'
tar -xzf spannerplan-ffi-0.1.0-alpha.2-aarch64-apple-darwin.tar.gz
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

During alpha, crates remain marked `publish = false` and releases do not publish
to crates.io.

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
gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs \
  --pattern 'spannerplan-core*.tgz' --pattern 'spannerplan-cli*.tgz'
npm install -g ./spannerplan-core-0.1.0-alpha.2.tgz ./spannerplan-cli-0.1.0-alpha.2.tgz
rendertree -mode plan < plan.yaml
```

Install both tarballs in the same `npm install` invocation. Do not install the
CLI tarball alone: `@spannerplan/core` is deliberately unpublished and cannot
be resolved from the npm registry.

### From a clone or submodule (builds WASM — requires Rust + wasm-pack)

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs/js
npm install
npm run build -w @spannerplan/core
```

For a submodule or existing clone, run the same `npm install` and workspace build
from its `js` directory.

## FFI bindings

Pattern for all FFI languages:

1. **Source** — clone or install the binding from git (`bindings/<lang>/`).
2. **Native library** — download and extract the matching versioned
   `spannerplan-ffi-<version>-<target-triple>.tar.gz` or `.zip` from the
   [GitHub Release](https://github.com/apstndb/spannerplan-rs/releases). Each
   archive contains the natural library filename, `spannerplan.h`, and
   `LICENSE`.
3. **Point the binding** — `SPANNERPLAN_FFI_LIB` or `SPANNERPLAN_FFI_DIR`.

### Python

```bash
pip install "spannerplan @ git+https://github.com/apstndb/spannerplan-rs@v0.1.0-alpha.2#subdirectory=bindings/python"

gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs --pattern \
  'spannerplan-ffi-0.1.0-alpha.2-aarch64-apple-darwin.tar.gz'
tar -xzf spannerplan-ffi-0.1.0-alpha.2-aarch64-apple-darwin.tar.gz
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.dylib"
```

### Java

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs/bindings/java

gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs --pattern \
  'spannerplan-ffi-0.1.0-alpha.2-x86_64-unknown-linux-gnu.tar.gz'
tar -xzf spannerplan-ffi-0.1.0-alpha.2-x86_64-unknown-linux-gnu.tar.gz
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.so"

mvn -q test
```

Add as a dependency via git submodule + local `mvn install`, or copy
`bindings/java` into your tree.

### .NET

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs

gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs --pattern \
  'spannerplan-ffi-0.1.0-alpha.2-x86_64-pc-windows-msvc.zip'
unzip spannerplan-ffi-0.1.0-alpha.2-x86_64-pc-windows-msvc.zip
export SPANNERPLAN_FFI_LIB="$PWD/spannerplan_ffi.dll"

dotnet test bindings/dotnet/SpannerPlan.sln
```

Reference `bindings/dotnet/src/SpannerPlan/SpannerPlan.csproj` from your
solution via project reference.

### Ruby

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs/bindings/ruby

gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs --pattern \
  'spannerplan-ffi-0.1.0-alpha.2-x86_64-apple-darwin.tar.gz'
tar -xzf spannerplan-ffi-0.1.0-alpha.2-x86_64-apple-darwin.tar.gz
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.dylib"

gem build spannerplan.gemspec
gem install ./spannerplan-0.1.0.alpha.2.gem
```

### PHP

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs/bindings/php

gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs --pattern \
  'spannerplan-ffi-0.1.0-alpha.2-x86_64-unknown-linux-gnu.tar.gz'
tar -xzf spannerplan-ffi-0.1.0-alpha.2-x86_64-unknown-linux-gnu.tar.gz
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.so"

composer install
php -d ffi.enable=true test_render.php
```

### C++

```bash
git clone --depth 1 --branch v0.1.0-alpha.2 https://github.com/apstndb/spannerplan-rs
cd spannerplan-rs

gh release download v0.1.0-alpha.2 --repo apstndb/spannerplan-rs --pattern \
  'spannerplan-ffi-0.1.0-alpha.2-aarch64-apple-darwin.tar.gz'
tar -xzf spannerplan-ffi-0.1.0-alpha.2-aarch64-apple-darwin.tar.gz
export SPANNERPLAN_FFI_LIB="$PWD/libspannerplan_ffi.dylib"

cmake -S bindings/cpp -B bindings/cpp/build
cmake --build bindings/cpp/build
```

## Verification

The tag workflow first verifies that the version encoded by the tag matches the
Cargo workspace version and both JavaScript package versions. It then builds,
archives, downloads, and checksums all release assets and runs the consumer
smoke tests. A successful
workflow deliberately leaves a verified **draft** GitHub Release; it never
publishes the release automatically.

Publishing is a separate, authorized manual step:

1. Prepare curated release notes outside this repository. Include the release
   highlights while preserving the draft's mechanical Rust, JavaScript, FFI,
   and Python install details. This project does not keep a per-version
   changelog in the repository.
2. Replace the draft body and inspect the complete release, including its
   assets and draft/prerelease state:

   ```bash
   gh release edit TAG --notes-file FILE
   gh release view TAG --json tagName,isDraft,isPrerelease,body,assets
   ```

3. Only after that inspection, publish the verified draft:

   ```bash
   gh release edit TAG --draft=false
   ```

After publication, verify all supported consumer paths with one command:

```bash
bash scripts/verify-release-consumers.sh TAG
```
