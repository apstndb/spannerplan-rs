/** Slim browser WASM (renderer + wire; no rendertree CLI entry). */
export interface WasmBindings {
  spannerplanRenderTreeTable(args: unknown[]): unknown;
  spannerplanRenderTreeTableWire(
    planWire: Uint8Array,
    mode: string | null | undefined,
    format: string | null | undefined,
    config: unknown,
  ): unknown;
  spannerplanInternalPlantreeRowsV1Alpha2(args: unknown[]): unknown;
  spannerplanInternalPlantreeRowsV1Alpha2Wire(
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
        spannerplanInternalPlantreeRowsV1Alpha2:
          mod.spannerplanInternalPlantreeRowsV1Alpha2,
        spannerplanInternalPlantreeRowsV1Alpha2Wire:
          mod.spannerplanInternalPlantreeRowsV1Alpha2Wire,
      };
    });
  }
  return browserBindings;
}
