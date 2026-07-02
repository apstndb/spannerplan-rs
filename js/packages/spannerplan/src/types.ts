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

export function isRendertreeRendered(
  response: RendertreeResponse,
): response is RendertreeResult {
  return response.kind === "rendered";
}
