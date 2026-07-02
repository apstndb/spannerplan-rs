# Distribution

This project is distributed through **GitHub Releases** and **git dependencies**.
We do not publish to crates.io, npmjs.org, PyPI, Maven Central, or NuGet for
now. Official package registries may be added later if discoverability demands
it.

## Primary channel: GitHub Releases

Tagged releases (`v*`) attach prebuilt artifacts:

| Asset | Consumer |
|-------|----------|
| `libspannerplan_ffi.{so,dylib,dll}` + `spannerplan.h` | FFI bindings (Python, Java, .NET, C++, Ruby, PHP) |
| `spannerplan-core-*.tgz` | `@spannerplan/core` (WASM-backed JS/TS library) |
| `spannerplan-cli-*.tgz` | `@spannerplan/cli` (`rendertree` npm binary) |
| `SHA256SUMS.txt` | Integrity verification |

Latest release: https://github.com/apstndb/spannerplan-rs/releases

## By language

### Rust

Git dependency (recommended):

```toml
[dependencies]
spannerplan = { git = "https://github.com/apstndb/spannerplan-rs", tag = "v0.1.0-alpha.1" }
```

Binary from source:

```bash
cargo install --git https://github.com/apstndb/spannerplan-rs --tag v0.1.0-alpha.1 spannerplan-cli
```

Crates are marked `publish = false` in this repo; `cargo publish` is not part
of the release workflow.

### JavaScript / TypeScript

Prebuilt tarball from a release (no Rust toolchain required):

```bash
gh release download v0.1.0-alpha.1 --repo apstndb/spannerplan-rs --pattern 'spannerplan-core*.tgz'
npm install ./spannerplan-core-0.1.0-alpha.1.tgz
```

`package.json`:

```json
{
  "dependencies": {
    "@spannerplan/core": "file:./vendor/spannerplan-core-0.1.0-alpha.1.tgz"
  }
}
```

Git dependency (builds WASM from source; requires Rust + wasm-pack):

```json
{
  "dependencies": {
    "@spannerplan/core": "github:apstndb/spannerplan-rs#v0.1.0-alpha.1&path:js/packages/spannerplan"
  }
}
```

### FFI bindings (Python, Java, .NET, Ruby, PHP, C++)

1. Install binding source from git (per-language `bindings/*` README).
2. Download the matching `libspannerplan_ffi.*` from the GitHub Release.
3. Set `SPANNERPLAN_FFI_LIB` or `SPANNERPLAN_FFI_DIR`.

Example (Python):

```bash
pip install "spannerplan @ git+https://github.com/apstndb/spannerplan-rs@v0.1.0-alpha.1#subdirectory=bindings/python"
export SPANNERPLAN_FFI_LIB=/path/to/libspannerplan_ffi.dylib
```

## Optional: GitHub Packages

For org-internal npm consumers, the Release workflow can publish to
[GitHub Packages](https://github.com/features/packages) (`npm.pkg.github.com`)
when triggered manually with **“Also publish npm tarballs to GitHub Packages”**
enabled.

Published scope: `@apstndb/spannerplan-core`, `@apstndb/spannerplan-cli`
(GitHub Packages requires the scope to match the repository owner).

```bash
# .npmrc (consumer)
@apstndb:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}

npm install @apstndb/spannerplan-core@0.1.0-alpha.1
```

Tag-push releases do **not** publish to GitHub Packages automatically; use the
workflow dispatch checkbox when you need registry hosting in addition to release
assets.

## Verification

After a release, smoke-test consumer installs:

```bash
bash scripts/verify-release-consumers.sh v0.1.0-alpha.1
```

CI runs a subset of these checks in the Release workflow (`verify-consumers` job).
