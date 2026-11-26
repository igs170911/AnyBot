import { type ChildProcess, spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

export interface EnvPorts {
  server_port: number;
  model_port: number;
  push_port: number;
  proxy_port: number;
  control_port: number;
}

const READY_PREFIX = "testenv ready ";
const READY_TIMEOUT_MS = 30_000;

export function portsFile(): string {
  return path.resolve(import.meta.dirname, "../../../target/scratch/testenv-ports.json");
}

export function readEnvPorts(): EnvPorts {
  const raw = readFileSyncSafe(portsFile());
  if (raw === undefined) {
    throw new Error("testenv ports file missing; global setup did not run");
  }
  return JSON.parse(raw) as EnvPorts;
}

function readFileSyncSafe(file: string): string | undefined {
  if (!existsSync(file)) {
    return undefined;
  }
  return readFileSync(file, "utf8");
}

const STOP_TIMEOUT_MS = 15_000;

function waitForExit(child: ChildProcess, timeoutMs: number): Promise<void> {
  return new Promise((resolve, reject) => {
    if (child.exitCode !== null || child.signalCode !== null) {
      resolve();
      return;
    }
    const timer = setTimeout(() => {
      reject(new Error(`testenv did not exit within ${timeoutMs} ms`));
    }, timeoutMs);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve();
    });
  });
}

async function stopChild(child: ChildProcess): Promise<void> {
  if (child.exitCode !== null || child.signalCode !== null) {
    return;
  }
  child.kill("SIGTERM");
  await waitForExit(child, STOP_TIMEOUT_MS);
}

function waitForReadyLine(child: ChildProcess): Promise<string> {
  return new Promise((resolve, reject) => {
    let text = "";
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) {
        return;
      }
      settled = true;
      void stopChild(child).finally(() => {
        reject(new Error(`testenv did not print a ready line within ${READY_TIMEOUT_MS} ms`));
      });
    }, READY_TIMEOUT_MS);
    child.stdout?.setEncoding("utf8");
    child.stdout?.on("data", (chunk: string) => {
      text += chunk;
      const line = text.split("\n").find((l) => l.startsWith(READY_PREFIX));
      if (line !== undefined && !settled) {
        settled = true;
        clearTimeout(timer);
        resolve(line.slice(READY_PREFIX.length));
      }
    });
    child.on("exit", (code) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timer);
      reject(new Error(`testenv exited early with code ${String(code)}`));
    });
  });
}

function testenvBin(): string {
  const repoRoot = path.resolve(import.meta.dirname, "../../..");
  const run = spawnSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  if (run.status !== 0) {
    throw new Error(`cargo metadata failed: ${String(run.stderr)}`);
  }
  const meta = JSON.parse(String(run.stdout)) as { target_directory: string };
  return path.join(meta.target_directory, "debug", "testenv");
}

export default async function globalSetup(): Promise<() => void> {
  const bin = testenvBin();
  if (!existsSync(bin)) {
    throw new Error(`testenv binary missing: ${bin} (run cargo build -p testenv first)`);
  }
  mkdirSync(path.dirname(portsFile()), { recursive: true });
  const child = spawn(bin, { stdio: ["ignore", "pipe", "pipe"] });
  const report = await waitForReadyLine(child);
  const ports = JSON.parse(report) as EnvPorts;
  writeFileSync(portsFile(), JSON.stringify(ports));
  return async () => {
    await stopChild(child);
  };
}
