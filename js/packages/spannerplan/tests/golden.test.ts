import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

import {
  isRenderError,
  renderRendertree,
  renderTreeTable,
  renderTreeTableWire,
} from "../src/index.js";
import { parsePlanText } from "../src/input-browser.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "../../../..");

function fixture(rel: string): string {
  return readFileSync(join(repoRoot, "testdata", rel), "utf8");
}

function golden(name: string): string {
  return readFileSync(join(repoRoot, "testdata/golden", `${name}.txt`), "utf8");
}

function wireFixture(name: string): Uint8Array {
  return readFileSync(join(repoRoot, "testdata/wire", `${name}_query_plan.bin`));
}

describe("reference renderer", () => {
  it("matches Go golden for dca plan/current", () => {
    const yaml = fixture("reference/dca.yaml");
    const result = renderTreeTable(yaml, "PLAN", "CURRENT");
    expect(isRenderError(result)).toBe(false);
    if (!isRenderError(result)) {
      expect(result.output).toBe(golden("dca_plan_current"));
    }
  });

  it("accepts JSON objects", () => {
    const yaml = fixture("reference/dca.yaml");
    const json = parse(yaml) as Record<string, unknown>;
    const fromYaml = renderTreeTable(yaml, "PLAN", "CURRENT");
    const fromJson = renderTreeTable(json, "PLAN", "CURRENT");
    expect(fromYaml).toEqual(fromJson);
  });

  it("host YAML parse matches WASM yaml path (browser slim input)", () => {
    const yaml = fixture("reference/dca.yaml");
    const parsed = parsePlanText(yaml);
    const fromYaml = renderTreeTable(yaml, "PLAN", "CURRENT");
    const fromHostYaml = renderTreeTable(parsed, "PLAN", "CURRENT");
    expect(fromHostYaml).toEqual(fromYaml);
  });
});

describe("wire renderer", () => {
  for (const [fixtureName, goldenName] of [
    ["dca", "dca_plan_current"],
    ["dcaplan", "dcaplan_plan_current"],
  ] as const) {
    it(`matches YAML golden for ${fixtureName} plan/current`, () => {
      const wire = wireFixture(fixtureName);
      const result = renderTreeTableWire(wire, "PLAN", "CURRENT");
      expect(isRenderError(result)).toBe(false);
      if (!isRenderError(result)) {
        expect(result.output).toBe(golden(goldenName));
      }
    });
  }

  it("wire and YAML paths agree on dca", () => {
    const yaml = fixture("reference/dca.yaml");
    const wire = wireFixture("dca");
    const fromYaml = renderTreeTable(yaml, "PLAN", "CURRENT");
    const fromWire = renderTreeTableWire(wire, "PLAN", "CURRENT");
    expect(fromYaml).toEqual(fromWire);
  });
});

describe("rendertree CLI path", () => {
  it("matches Go golden for dca plan mode", () => {
    const input = fixture("reference/dca.yaml");
    const result = renderRendertree(input, ["-mode", "plan"]);
    expect(result.kind).toBe("rendered");
    if (result.kind === "rendered") {
      expect(result.output).toBe(golden("dca_rendertree_plan"));
    }
  });

  it("returns usage kind for unknown flags", () => {
    const input = fixture("reference/dca.yaml");
    const result = renderRendertree(input, ["-not-a-flag"]);
    expect(result.kind).toBe("usage");
  });
});
