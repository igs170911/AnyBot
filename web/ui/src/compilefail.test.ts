import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import path from "node:path";
import { describe, expect, it } from "vitest";

const require = createRequire(import.meta.url);

function tscPath(): string {
  const pkg = require.resolve("typescript/package.json");
  return path.join(path.dirname(pkg), "bin", "tsc");
}

describe("generated message code types", () => {
  it("tsc rejects a message code that is not in en.ftl", () => {
    const config = path.resolve(process.cwd(), "tests/compilefail/tsconfig.json");
    const run = spawnSync(process.execPath, [tscPath(), "--noEmit", "-p", config], {
      encoding: "utf8",
    });
    expect(run.status, run.stderr).not.toBe(0);
    expect(run.stdout + run.stderr).toContain("not-a-code");
  });

  it("tsc accepts the generated codes file", () => {
    const run = spawnSync(process.execPath, [tscPath(), "--noEmit", "-p", "tsconfig.json"], {
      encoding: "utf8",
    });
    expect(run.status, run.stderr).toBe(0);
  });
});
