import { createReadStream, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, relative, resolve } from "node:path";

const root = resolve(process.argv[2] ?? ".");
const wasmMime = process.env.RELEASE_SMOKE_WASM_MIME ?? "application/wasm";
const mimeTypes = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": wasmMime,
  ".yaml": "text/yaml; charset=utf-8",
};

const server = createServer((request, response) => {
  try {
    const requestPath = new URL(request.url ?? "/", "http://127.0.0.1").pathname;
    const candidate = resolve(root, `.${normalize(requestPath)}`);
    if (relative(root, candidate).startsWith("..")) {
      response.writeHead(403).end("forbidden");
      return;
    }
    const file = statSync(candidate).isDirectory() ? join(candidate, "index.html") : candidate;
    response.writeHead(200, { "Content-Type": mimeTypes[extname(file)] ?? "application/octet-stream" });
    createReadStream(file).pipe(response);
  } catch {
    response.writeHead(404).end("not found");
  }
});

server.listen(0, "127.0.0.1", () => {
  console.log(server.address().port);
});

process.once("SIGTERM", () => server.close(() => process.exit(0)));
