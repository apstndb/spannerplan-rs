export interface WasmBindings {
  spannerplanRenderTreeTable(args: unknown[]): unknown;
  spannerplanRenderTreeTableWire(
    planWire: Uint8Array,
    mode: string | null | undefined,
    format: string | null | undefined,
    config: unknown,
  ): unknown;
  spannerplanRenderRendertree(
    input: Uint8Array,
    args: string[],
  ): unknown;
}

let browserBindings: Promise<WasmBindings> | null = null;

/** Load WASM bindings for browsers and bundlers (async init). */
export async function getBrowserWasm(): Promise<WasmBindings> {
  if (!browserBindings) {
    browserBindings = import("../wasm/spannerplan_wasm.js").then((mod) => ({
      spannerplanRenderTreeTable: mod.spannerplanRenderTreeTable,
      spannerplanRenderTreeTableWire: mod.spannerplanRenderTreeTableWire,
      spannerplanRenderRendertree: mod.spannerplanRenderRendertree,
    }));
  }
  return browserBindings;
}
