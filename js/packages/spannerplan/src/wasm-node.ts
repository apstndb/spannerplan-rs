import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

import type { WasmBindings as BrowserWasmBindings } from "./wasm-browser.js";

/** Full Node WASM (yaml + wire + rendertree CLI entry). */
export interface NodeWasmBindings extends BrowserWasmBindings {
  spannerplanRenderRendertree(input: Uint8Array, args: string[]): unknown;
}

let nodeBindings: NodeWasmBindings | null = null;

function loadNodeBindings(): NodeWasmBindings {
  if (!nodeBindings) {
    const require = createRequire(fileURLToPath(import.meta.url));
    nodeBindings = require("../wasm-node/spannerplan_wasm.js") as NodeWasmBindings;
  }
  return nodeBindings;
}

/** Load WASM bindings for Node.js (sync after first call). */
export function getNodeWasm(): NodeWasmBindings {
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
