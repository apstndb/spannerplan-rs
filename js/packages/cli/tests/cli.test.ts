import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { runCli } from "../src/main.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "../../../..");

function fixture(rel: string): Buffer {
  return readFileSync(join(repoRoot, "testdata", rel));
}

function golden(name: string): string {
  return readFileSync(join(repoRoot, "testdata/golden", `${name}.txt`), "utf8");
}

function captureStdout(run: () => number): { code: number; stdout: string } {
  const chunks: string[] = [];
  const original = process.stdout.write.bind(process.stdout);
  process.stdout.write = ((chunk: string | Uint8Array) => {
    chunks.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
    return true;
  }) as typeof process.stdout.write;
  try {
    const code = run();
    return { code, stdout: chunks.join("") };
  } finally {
    process.stdout.write = original;
  }
}

describe("rendertree bin", () => {
  it("matches golden for dca plan mode", () => {
    const { code, stdout } = captureStdout(() =>
      runCli(["-mode", "plan"], fixture("reference/dca.yaml")),
    );
    expect(code).toBe(0);
    expect(stdout).toBe(golden("dca_rendertree_plan"));
  });

  it("exits 0 and renders plan mode", () => {
    const code = runCli(["-mode", "plan"], fixture("reference/dca.yaml"));
    expect(code).toBe(0);
  });

  it("exits 2 on unknown flag", () => {
    const code = runCli(["-bogus"], fixture("reference/dca.yaml"));
    expect(code).toBe(2);
  });

  it("exits 0 for help", () => {
    const code = runCli(["-h"], Buffer.alloc(0));
    expect(code).toBe(0);
  });
});
