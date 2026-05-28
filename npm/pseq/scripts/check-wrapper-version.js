#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(here, "..");
const packageJson = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8"));
const wrapper = join(packageRoot, "bin", "pseq.js");

const result = spawnSync(process.execPath, [wrapper, "--version"], {
  cwd: packageRoot,
  encoding: "utf8",
});

if (result.status !== 0 || result.error) {
  fail(
    [
      "failed to run pseq wrapper",
      result.error?.message,
      result.stdout.trim(),
      result.stderr.trim(),
    ].filter(Boolean).join("\n"),
  );
}

const output = `${result.stdout}${result.stderr}`.trim();
if (!output.includes(packageJson.version)) {
  fail(`pseq wrapper reported ${JSON.stringify(output)}, expected version ${packageJson.version}`);
}

console.log(output);

function fail(message) {
  console.error(message);
  process.exit(1);
}
