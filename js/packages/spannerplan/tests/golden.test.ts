import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

import {
  internalPlantreeRowsV1Alpha2,
  internalPlantreeRowsV1Alpha2OrThrow,
  internalPlantreeRowsV1Alpha2Wire,
  isInternalPlantreeRowsErrorV1Alpha2,
  isRenderError,
  renderRendertree,
  renderTreeTable,
  renderTreeTableWire,
} from "../src/index.js";
import { parsePlanText } from "../src/input-browser.js";
import { parseInternalPlantreeRowsResponseV1Alpha2 } from "../src/plantree.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "../../../..");

function fixture(rel: string): string {
  return readFileSync(join(repoRoot, "testdata", rel), "utf8");
}

function golden(name: string): string {
  return readFileSync(join(repoRoot, "testdata/golden", `${name}.txt`), "utf8");
}

function structuredGolden(name: string): unknown {
  return JSON.parse(
    readFileSync(join(repoRoot, "testdata/golden", `${name}.json`), "utf8"),
  );
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

describe("viewer-internal Plantree v1alpha2 rows", () => {
  it("rejects malformed runtime response DTOs", () => {
    const valid = structuredGolden("dca_plantree_rows_current") as {
      contractVersion: number;
      rows: unknown[];
    };
    const firstRow = valid.rows[0] as Record<string, unknown>;
    const firstLink = (firstRow.scalarChildLinks as unknown[])[0] as Record<string, unknown>;

    expect(parseInternalPlantreeRowsResponseV1Alpha2({ rows: [] })).toEqual({
      error: "unsupported WASM Plantree contract version",
    });
    expect(parseInternalPlantreeRowsResponseV1Alpha2({ contractVersion: 1, rows: [] })).toEqual({
      error: "unsupported WASM Plantree contract version",
    });
    expect(parseInternalPlantreeRowsResponseV1Alpha2({ contractVersion: 2 })).toEqual({
      error: "unexpected WASM Plantree rows response",
    });
    expect(
      parseInternalPlantreeRowsResponseV1Alpha2({
        contractVersion: 2,
        rows: [{ ...firstRow, nodeId: "not-a-number" }],
      }),
    ).toEqual({ error: "unexpected WASM Plantree row response" });
    expect(
      parseInternalPlantreeRowsResponseV1Alpha2({
        contractVersion: 2,
        rows: [
          {
            ...firstRow,
            scalarChildLinks: [{ ...firstLink, isPredicate: "true" }],
          },
        ],
      }),
    ).toEqual({ error: "unexpected WASM Plantree row response" });
    expect(parseInternalPlantreeRowsResponseV1Alpha2({ error: 42 })).toEqual({
      error: "unexpected WASM Plantree error response",
    });

    const projected = parseInternalPlantreeRowsResponseV1Alpha2({
      contractVersion: 2,
      rows: [
        {
          ...firstRow,
          executionStats: { rows: 10 },
          scalarChildLinks: [{ ...firstLink, occurrenceId: "future-field" }],
        },
      ],
    });
    expect(projected).toEqual({
      contractVersion: 2,
      rows: [
        {
          rowId: firstRow.rowId,
          parentRowId: firstRow.parentRowId,
          nodeId: firstRow.nodeId,
          treePart: firstRow.treePart,
          nodeText: firstRow.nodeText,
          displayName: firstRow.displayName,
          predicates: firstRow.predicates,
          scalarChildLinks: [
            {
              type: firstLink.type,
              variable: firstLink.variable,
              description: firstLink.description,
              displayName: firstLink.displayName,
              childIndex: firstLink.childIndex,
              isPredicate: firstLink.isPredicate,
            },
          ],
        },
      ],
    });
  });

  for (const [fixturePath, goldenName] of [
    ["reference/dca.yaml", "dca_plantree_rows_current"],
    [
      "reference/distributed_cross_apply.yaml",
      "dcaplan_plantree_rows_current",
    ],
  ] as const) {
    it(`matches the Go-derived ${goldenName} projection`, () => {
      const response = internalPlantreeRowsV1Alpha2(fixture(fixturePath));
      expect(isInternalPlantreeRowsErrorV1Alpha2(response)).toBe(false);
      if (!isInternalPlantreeRowsErrorV1Alpha2(response)) {
        expect(response).toEqual(structuredGolden(goldenName));
      }
    });
  }

  it("accepts YAML text and JSON objects equivalently", () => {
    const yaml = fixture("reference/dca.yaml");
    const json = parse(yaml) as Record<string, unknown>;
    expect(internalPlantreeRowsV1Alpha2(json)).toEqual(internalPlantreeRowsV1Alpha2(yaml));
  });

  it("matches the wire projection", () => {
    const yaml = fixture("reference/dca.yaml");
    const wire = wireFixture("dca");
    expect(internalPlantreeRowsV1Alpha2Wire(wire)).toEqual(internalPlantreeRowsV1Alpha2(yaml));
  });

  it("returns the error envelope for an invalid format", () => {
    const response = internalPlantreeRowsV1Alpha2(fixture("reference/dca.yaml"), "bad-format" as never);
    expect(response).toEqual({ error: "unknown format: bad-format" });
  });

  it("rejects the error envelope through the throwing helper", async () => {
    await expect(
      internalPlantreeRowsV1Alpha2OrThrow(fixture("reference/dca.yaml"), "bad-format" as never),
    ).rejects.toThrow("unknown format: bad-format");
  });

  it("keeps predicate classification on scalar child links", async () => {
    const rows = await internalPlantreeRowsV1Alpha2OrThrow(fixture("reference/dca.yaml"));
    const predicateLinks = rows.flatMap((row) =>
      row.scalarChildLinks.filter((link) => link.isPredicate),
    );
    expect(predicateLinks.length).toBeGreaterThan(0);
    expect(predicateLinks.every((link) => link.displayName === "Function")).toBe(true);
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
