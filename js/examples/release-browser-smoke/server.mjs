import { createReadStream, renameSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, relative, resolve } from "node:path";

const root = resolve(process.argv[2] ?? ".");
const resultFile = process.argv[3];
if (!resultFile) {
  throw new Error("release browser smoke server requires a result-file argument");
}
const resultFilePath = resolve(resultFile);
const resultEndpoint = "/__release-smoke-result";
const maxResultBytes = 64 * 1024;
const wasmMime = process.env.RELEASE_SMOKE_WASM_MIME ?? "application/wasm";
const mimeTypes = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": wasmMime,
  ".yaml": "text/yaml; charset=utf-8",
};

function writeResult(request, response) {
  let size = 0;
  const chunks = [];
  let finished = false;
  const fail = (status, message) => {
    if (finished) return;
    finished = true;
    response.writeHead(status, { "Content-Type": "text/plain; charset=utf-8" }).end(message);
  };

  request.on("data", (chunk) => {
    if (finished) return;
    size += chunk.length;
    if (size > maxResultBytes) {
      fail(413, "result payload too large");
      request.destroy();
      return;
    }
    chunks.push(chunk);
  });
  request.on("error", () => fail(400, "result request failed"));
  request.on("end", () => {
    if (finished) return;
    try {
      const body = Buffer.concat(chunks).toString("utf8");
      JSON.parse(body);
      const temporary = `${resultFilePath}.partial`;
      writeFileSync(temporary, body, { encoding: "utf8", mode: 0o600 });
      renameSync(temporary, resultFilePath);
      finished = true;
      response.writeHead(204).end();
    } catch {
      fail(400, "invalid result payload");
    }
  });
}

const server = createServer((request, response) => {
  try {
    const requestPath = new URL(request.url ?? "/", "http://127.0.0.1").pathname;
    if (requestPath === resultEndpoint) {
      if (request.method !== "POST") {
        response.writeHead(405, { Allow: "POST" }).end("method not allowed");
        return;
      }
      writeResult(request, response);
      return;
    }
    if (request.method !== "GET" && request.method !== "HEAD") {
      response.writeHead(405, { Allow: "GET, HEAD, POST" }).end("method not allowed");
      return;
    }
    const candidate = resolve(root, `.${normalize(requestPath)}`);
    if (relative(root, candidate).startsWith("..")) {
      response.writeHead(403).end("forbidden");
      return;
    }
    const file = statSync(candidate).isDirectory() ? join(candidate, "index.html") : candidate;
    response.writeHead(200, { "Content-Type": mimeTypes[extname(file)] ?? "application/octet-stream" });
    if (request.method === "HEAD") {
      response.end();
    } else {
      createReadStream(file).pipe(response);
    }
  } catch {
    response.writeHead(404).end("not found");
  }
});

server.listen(0, "127.0.0.1", () => {
  console.log(server.address().port);
});

process.once("SIGTERM", () => server.close(() => process.exit(0)));
