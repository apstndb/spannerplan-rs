/** How to render the query plan output. */
export type RenderMode = "AUTO" | "PLAN" | "PROFILE";

/** Table formatting preset. */
export type Format = "TRADITIONAL" | "CURRENT" | "COMPACT";

/** Appendix section identifiers (comma-separated in CLI `-print`). */
export type PrintSection =
  | "predicates"
  | "ordering"
  | "aggregate"
  | "typed"
  | "full";

/** Appendix print presets accepted by `-print`. */
export type PrintPreset = "basic" | "enhanced" | "full" | "none";

/**
 * Optional rendering behavior passed to the reference renderer.
 *
 * JSON shape matches Rust `RenderConfig` and FFI/WASM `config` arguments
 * (camelCase). Schema: `schema/render-config.schema.json` in the repo root.
 */
export interface RenderConfig {
  wrapWidth?: number;
  hangingIndent?: boolean;
  /** `undefined` = default (predicates); `[]` = no appendix. */
  printSections?: PrintSection[] | null;
  showScalarVars?: boolean;
  resolveScalarVars?: boolean;
  resolveScalarVarsRecursive?: boolean;
  disallowUnknownStats?: boolean;
}

export interface RenderResult {
  output: string;
}

export interface RenderError {
  error: string;
}

export type RenderResponse = RenderResult | RenderError;

/** @internal Bundled viewer formatting for the Plantree v1alpha2 contract. */
export interface InternalPlantreeConfigV1Alpha2 {
  wrapWidth?: number;
  hangingIndent?: boolean;
  disallowUnknownStats?: boolean;
}

/** @internal A scalar child link in the bundled viewer contract. */
export interface InternalPlantreeChildLinkV1Alpha2 {
  type: string;
  variable: string;
  description: string;
  displayName: string;
  childIndex: number;
  /** Classified by the query-plan API, not inferred from rendered text. */
  isPredicate: boolean;
}

/** @internal One occurrence from the bundled viewer's pre-order traversal. */
export interface InternalPlantreeRowV1Alpha2 {
  rowId: string;
  parentRowId: string | null;
  nodeId: number;
  treePart: string;
  nodeText: string;
  displayName: string;
  predicates: string[];
  scalarChildLinks: InternalPlantreeChildLinkV1Alpha2[];
}

/** @internal v1alpha2 success envelope. Numeric wire revision is 2. */
export interface InternalPlantreeRowsResultV1Alpha2 {
  contractVersion: 2;
  rows: InternalPlantreeRowV1Alpha2[];
}

/** @internal */
export interface InternalPlantreeRowsErrorV1Alpha2 {
  error: string;
}

/** @internal */
export type InternalPlantreeRowsResponseV1Alpha2 =
  | InternalPlantreeRowsResultV1Alpha2
  | InternalPlantreeRowsErrorV1Alpha2;

export interface RendertreeResult {
  kind: "rendered";
  output: string;
}

export interface RendertreeHelp {
  kind: "help";
  stderr: string;
}

export interface RendertreeUsageError {
  kind: "usage";
  stderr: string;
  error: string;
}

export interface RendertreeFailed {
  kind: "failed";
  stderr: string;
  error: string;
}

export type RendertreeResponse =
  | RendertreeResult
  | RendertreeHelp
  | RendertreeUsageError
  | RendertreeFailed;

/** Accepted plan input shapes for the reference renderer. */
export type PlanInput = string | Uint8Array | Record<string, unknown>;

/** Raw stdin bytes for the `rendertree` CLI path (YAML or JSON). */
export type BytesInput = Uint8Array | Buffer | string;

export function isRenderError(
  response: RenderResponse,
): response is RenderError {
  return "error" in response;
}

/** @internal True when the bundled Plantree response is an error envelope. */
export function isInternalPlantreeRowsErrorV1Alpha2(
  response: InternalPlantreeRowsResponseV1Alpha2,
): response is InternalPlantreeRowsErrorV1Alpha2 {
  return "error" in response;
}

export function isRendertreeRendered(
  response: RendertreeResponse,
): response is RendertreeResult {
  return response.kind === "rendered";
}
