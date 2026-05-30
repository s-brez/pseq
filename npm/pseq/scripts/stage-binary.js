#!/usr/bin/env node
import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  currentPlatformArch,
  requiredBinaryFor,
  requiredBinaryPath,
} from "./platforms.js";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(here, "..");
const repoRoot = resolve(packageRoot, "../..");

const options = parseArgs(process.argv.slice(2));
const platformArch = options.platform ?? currentPlatformArch();
const entry = requiredBinaryFor(platformArch);
if (!entry) {
  console.error(`unsupported pseq package platform: ${platformArch}`);
  process.exit(1);
}

const source = options.source
  ? resolve(repoRoot, options.source)
  : defaultSource(entry.binary);

if (!existsSync(source)) {
  console.error(`pseq binary source not found: ${source}`);
  process.exit(1);
}

const destination = requiredBinaryPath(packageRoot, entry);
mkdirSync(dirname(destination), { recursive: true });
copyFileSync(source, destination);
if (!entry.binary.endsWith(".exe")) {
  chmodSync(destination, 0o755);
}

console.log(destination);

function parseArgs(args) {
  const parsed = {};
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--platform") {
      parsed.platform = requireValue(args, ++index, arg);
    } else if (arg === "--source") {
      parsed.source = requireValue(args, ++index, arg);
    } else {
      console.error(`unknown argument: ${arg}`);
      process.exit(1);
    }
  }
  return parsed;
}

function requireValue(args, index, flag) {
  const value = args[index];
  if (!value) {
    console.error(`missing value for ${flag}`);
    process.exit(1);
  }
  return value;
}

function defaultSource(binaryName) {
  const candidates = [
    join(repoRoot, "target", "release", binaryName),
    join(repoRoot, "target", "debug", binaryName),
  ];
  const source = candidates.find((candidate) => existsSync(candidate));
  if (source) {
    return source;
  }
  return candidates[0];
}
