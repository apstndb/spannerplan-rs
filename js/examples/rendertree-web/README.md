# rendertree-web

Minimal browser sample for [`@spannerplan/core`](../../packages/spannerplan).
Paste or upload a Spanner query plan (YAML or JSON) and view the ASCII table in
the page.

Inspired by [apstndb/rendertree-web](https://github.com/apstndb/rendertree-web);
this version uses the Rust/WASM renderer from this monorepo.

## Run locally

From the `js/` workspace root:

```bash
npm install
npm run build -w @spannerplan/core
npm run dev -w rendertree-web
```

Open the URL Vite prints (default `http://localhost:5173`).

## Build static assets

```bash
npm run build -w rendertree-web
npm run preview -w rendertree-web
```

## Notes

- YAML is converted to JSON in the page before calling WASM (the core browser
  entry accepts JSON text or objects).
- This sample uses the **reference** renderer API (`renderTreeTable`), not the
  full `rendertree` CLI layout. For CLI parity in Node, use
  [`@spannerplan/cli`](../../packages/cli).
