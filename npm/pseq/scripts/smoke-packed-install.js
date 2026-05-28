#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  currentPlatformArch,
  requiredBinaryFor,
  requiredBinaryPath,
} from "./platforms.js";

const here = dirname(fileURLToPath(import.meta.url));
const options = parseArgs(process.argv.slice(2));
const packageRoot = resolve(options.packageRoot ?? join(here, ".."));
const repoRoot = resolve(options.repoRoot ?? join(packageRoot, "../.."));
const packageJson = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8"));
const platformArch = currentPlatformArch();
const platformEntry = requiredBinaryFor(platformArch);

if (!platformEntry) {
  fail(`unsupported smoke-test platform: ${platformArch}`);
}

const platformBinary = requiredBinaryPath(packageRoot, platformEntry);
if (!existsSync(platformBinary)) {
  fail(
    `missing current-platform binary ${platformBinary}; run npm run stage:binary before packed-install smoke tests`,
  );
}

const tarball = options.tarball ? resolve(options.tarball) : packPackage();
const tempRoot = mkdtempSync(join(tmpdir(), "pseq-packed-install-"));

try {
  writeFileSync(
    join(tempRoot, "package.json"),
    JSON.stringify({ private: true, name: "pseq-packed-install-smoke" }, null, 2),
  );
  run("npm", [
    "install",
    tarball,
    "--ignore-scripts",
    "--no-audit",
    "--no-fund",
    "--package-lock=false",
  ], tempRoot);

  const shim = process.platform === "win32" ? "pseq.cmd" : "pseq";
  const pseq = join(tempRoot, "node_modules", ".bin", shim);
  const version = run(pseq, ["--version"], tempRoot);
  if (!version.stdout.includes(packageJson.version)) {
    fail(`installed pseq --version did not include ${packageJson.version}: ${version.stdout.trim()}`);
  }

  if (!options.quiet) {
    console.log(`packed install smoke passed for ${platformArch}`);
  }
} finally {
  if (!options.keep) {
    rmSync(tempRoot, { recursive: true, force: true });
    if (!options.tarball) {
      rmSync(tarball, { force: true });
    }
  }
}

function packPackage() {
  run("node", [
    "scripts/verify-release.js",
    "--package-root",
    packageRoot,
    "--repo-root",
    repoRoot,
    "--skip-pack",
    "--quiet",
  ], packageRoot);
  const result = run("npm", ["pack", "--json", "--ignore-scripts"], packageRoot);
  let packs;
  try {
    packs = JSON.parse(result.stdout);
  } catch (error) {
    fail(`npm pack did not return JSON: ${error.message}`);
  }

  const filename = packs?.[0]?.filename;
  if (!filename) {
    fail("npm pack did not report a tarball filename");
  }
  return join(packageRoot, filename);
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  if (result.status !== 0 || result.error) {
    fail(
      [
        `command failed: ${command} ${args.join(" ")}`,
        result.error?.message,
        result.stdout.trim(),
        result.stderr.trim(),
      ].filter(Boolean).join("\n"),
    );
  }
  return result;
}

function parseArgs(args) {
  const parsed = {
    keep: false,
    quiet: false,
  };
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--keep") {
      parsed.keep = true;
    } else if (arg === "--quiet") {
      parsed.quiet = true;
    } else if (arg === "--package-root") {
      parsed.packageRoot = requireValue(args, ++index, arg);
    } else if (arg === "--repo-root") {
      parsed.repoRoot = requireValue(args, ++index, arg);
    } else if (arg === "--tarball") {
      parsed.tarball = requireValue(args, ++index, arg);
    } else {
      fail(`unknown argument: ${arg}`);
    }
  }
  return parsed;
}

function requireValue(args, index, flag) {
  const value = args[index];
  if (!value) {
    fail(`missing value for ${flag}`);
  }
  return value;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
