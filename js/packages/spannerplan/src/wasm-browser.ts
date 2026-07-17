/** Slim browser WASM (renderer + wire; no rendertree CLI entry). */
export interface WasmBindings {
  spannerplanRenderTreeTable(args: unknown[]): unknown;
  spannerplanRenderTreeTableWire(
    planWire: Uint8Array,
    mode: string | null | undefined,
    format: string | null | undefined,
    config: unknown,
  ): unknown;
  spannerplanPlantreeRows(args: unknown[]): unknown;
  spannerplanPlantreeRowsWire(
    planWire: Uint8Array,
    format: string | null | undefined,
    config: unknown,
  ): unknown;
}

let browserBindings: Promise<WasmBindings> | null = null;

/** Load WASM bindings for browsers and bundlers (async init). */
export async function getBrowserWasm(): Promise<WasmBindings> {
  if (!browserBindings) {
    browserBindings = import("../wasm/spannerplan_wasm.js").then(async (mod) => {
      await mod.default();
      return {
        spannerplanRenderTreeTable: mod.spannerplanRenderTreeTable,
        spannerplanRenderTreeTableWire: mod.spannerplanRenderTreeTableWire,
        spannerplanPlantreeRows: mod.spannerplanPlantreeRows,
        spannerplanPlantreeRowsWire: mod.spannerplanPlantreeRowsWire,
      };
    });
  }
  return browserBindings;
}
