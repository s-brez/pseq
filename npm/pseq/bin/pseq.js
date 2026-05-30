#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "../../..");
const binaryName = process.platform === "win32" ? "pseq.exe" : "pseq";
const platformArch = `${process.platform}-${process.arch}`;
const candidates = [
  join(here, platformArch, binaryName),
  sourceCheckoutBinary(),
].filter(Boolean);

const binary = candidates.find((candidate) => existsSync(candidate));
if (!binary) {
  console.error(
    `pseq binary not found for ${platformArch}. The npm package should include a compiled platform binary; source checkout users can run \`cargo build\` from the repository root.`,
  );
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`failed to run pseq binary: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 1);

function newestExisting(paths) {
  return paths
    .filter((candidate) => existsSync(candidate))
    .sort((left, right) => statSync(right).mtimeMs - statSync(left).mtimeMs)[0];
}

function sourceCheckoutBinary() {
  if (!isSourceCheckout()) {
    return undefined;
  }

  return newestExisting([
    join(repoRoot, "target", "release", binaryName),
    join(repoRoot, "target", "debug", binaryName),
  ]);
}

function isSourceCheckout() {
  return (
    here === join(repoRoot, "npm", "pseq", "bin") &&
    existsSync(join(repoRoot, "Cargo.toml")) &&
    existsSync(join(repoRoot, "src", "main.rs"))
  );
}
