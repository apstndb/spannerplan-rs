# @spannerplan/cli

Node.js `rendertree` command-line tool for Cloud Spanner query plans. Reads YAML
or JSON from stdin and prints the ASCII plan table to stdout.

Depends on [`@spannerplan/core`](../spannerplan) (WASM-backed).

## Install

```bash
npm install -g @spannerplan/cli
```

From this monorepo:

```bash
cd js && npm install && npm run build
npx rendertree -mode plan < ../../testdata/reference/dca.yaml
```

## Usage

Flags mirror the Go/Rust `rendertree` tool where implemented:

```
-mode string          AUTO, PLAN, or PROFILE (default AUTO)
-print string         appendix preset or sections (default basic)
-compact              compact operator tree
-wrap-width int       wrap Operator column (0 = off)
-hanging-indent       hang wrapped lines after [Input]/[Map] prefixes
-show-vars            show scalar variable assignments
-resolve-vars         resolve scalar variable aliases (experimental)
-resolve-vars-recursive
-disallow-unknown-stats
-execution-method     angle or raw
-target-metadata      on or raw
-known-flag           label or raw
-help
```

Exit codes: `0` success, `2` usage/flag errors (matches Rust CLI), `1` other failures.

## Not yet implemented (v1)

- Custom columns (`--custom`, `--custom-column`, `--custom-file`) — deferred (see `DESIGN.md` §12)
- `--inline-stats`

## Tests

```bash
npm test -w @spannerplan/cli
```
