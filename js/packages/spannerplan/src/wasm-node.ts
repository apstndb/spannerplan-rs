import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

import type { WasmBindings } from "./wasm-browser.js";

let nodeBindings: WasmBindings | null = null;

function loadNodeBindings(): WasmBindings {
  if (!nodeBindings) {
    const require = createRequire(fileURLToPath(import.meta.url));
    nodeBindings = require("../wasm-node/spannerplan_wasm.js") as WasmBindings;
  }
  return nodeBindings;
}

/** Load WASM bindings for Node.js (sync after first call). */
export function getNodeWasm(): WasmBindings {
  return loadNodeBindings();
}

/** True when running under Node.js. */
export function isNodeRuntime(): boolean {
  return (
    typeof process !== "undefined" &&
    Boolean(process.versions?.node) &&
    process.versions.node.length > 0
  );
}

export { getBrowserWasm } from "./wasm-browser.js";
export type { WasmBindings } from "./wasm-browser.js";
