import { renderRendertree } from "@spannerplan/core";

/**
 * Run the rendertree CLI. Returns a process exit code (2 on usage errors).
 */
export function runCli(args: string[], stdin: Uint8Array): number {
  const result = renderRendertree(stdin, args);

  if (result.kind === "help") {
    process.stderr.write(result.stderr);
    return 0;
  }

  if (result.kind === "usage") {
    if (result.stderr) {
      process.stderr.write(result.stderr);
    } else if (result.error) {
      process.stderr.write(`${result.error}\n`);
    }
    return 2;
  }

  if (result.kind === "failed") {
    if (result.stderr) {
      process.stderr.write(result.stderr);
    }
    if (result.error) {
      process.stderr.write(`${result.error}\n`);
    }
    return 1;
  }

  process.stdout.write(result.output);
  return 0;
}
