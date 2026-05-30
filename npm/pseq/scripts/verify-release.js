#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { requiredBinaries, requiredBinaryPath } from "./platforms.js";

const here = dirname(fileURLToPath(import.meta.url));
const options = parseArgs(process.argv.slice(2));
const packageRoot = resolve(options.packageRoot ?? join(here, ".."));
const repoRoot = resolve(options.repoRoot ?? join(packageRoot, "../.."));
const packageJsonPath = join(packageRoot, "package.json");
const cargoTomlPath = join(repoRoot, "Cargo.toml");
const cargoLockPath = join(repoRoot, "Cargo.lock");
const errors = [];

const packageJson = readJson(packageJsonPath);
const cargoToml = readText(cargoTomlPath);
const cargoLock = readText(cargoLockPath);
const cargoVersion = cargoPackageField(cargoToml, "version");
const cargoDescription = cargoPackageField(cargoToml, "description");
const cargoLockVersion = cargoLockPackageField(cargoLock, "pseq", "version");
const releaseVersion = options.tag
  ? versionFromTag(options.tag)
  : options.version;

check(packageJson.name === "@s-brez/pseq", "package name must be @s-brez/pseq");
check(packageJson.version === cargoVersion, "npm package version must match Cargo package version");
check(
  cargoLockVersion === cargoVersion,
  "Cargo.lock pseq package version must match Cargo package version",
);
if (releaseVersion) {
  check(
    releaseVersion === cargoVersion,
    `release version ${releaseVersion} must match Cargo package version ${cargoVersion}`,
  );
}
check(
  packageJson.description === cargoDescription,
  "npm package description must match Cargo package description",
);
check(packageJson.type === "module", "npm package must use ESM");
check(
  packageJson.repository?.url === "git+https://github.com/s-brez/pseq.git",
  "npm package repository URL must match the GitHub repository",
);
check(
  packageJson.repository?.directory === "npm/pseq",
  "npm package repository directory must be npm/pseq",
);
check(packageJson.bin?.pseq === "bin/pseq.js", "npm package must expose the pseq bin");
check(
  Array.isArray(packageJson.files) && packageJson.files.includes("bin/"),
  "npm package files must include bin/",
);

const wrapperPath = join(packageRoot, "bin", "pseq.js");
check(existsSync(wrapperPath), "bin/pseq.js must exist");
if (existsSync(wrapperPath)) {
  const wrapper = readText(wrapperPath);
  check(wrapper.startsWith("#!/usr/bin/env node"), "bin/pseq.js must have a node shebang");
  checkExecutable(wrapperPath, "bin/pseq.js");
}

if (!options.skipBinaries) {
  for (const entry of requiredBinaries) {
    const path = requiredBinaryPath(packageRoot, entry);
    check(existsSync(path), `missing required binary: ${displayPackagePath(path)}`);
    if (existsSync(path)) {
      check(statSync(path).isFile(), `required binary must be a file: ${displayPackagePath(path)}`);
      if (!entry.binary.endsWith(".exe")) {
        checkExecutable(path, displayPackagePath(path));
      }
    }
  }
}

if (!options.skipPack) {
  verifyDryRunPackContents();
}

if (errors.length > 0) {
  console.error("release package verification failed:");
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

if (!options.quiet) {
  console.log("release package verification passed");
}

function verifyDryRunPackContents() {
  const result = spawnSync("npm", ["pack", "--dry-run", "--json", "--ignore-scripts"], {
    cwd: packageRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    errors.push(
      [
        "npm pack --dry-run failed",
        result.stdout.trim(),
        result.stderr.trim(),
      ].filter(Boolean).join(": "),
    );
    return;
  }

  let packs;
  try {
    packs = JSON.parse(result.stdout);
  } catch (error) {
    errors.push(`npm pack --dry-run did not return JSON: ${error.message}`);
    return;
  }

  const files = packs?.[0]?.files;
  if (!Array.isArray(files)) {
    errors.push("npm pack --dry-run JSON did not include files");
    return;
  }

  const expectedFiles = new Set([
    "README.md",
    "package.json",
    "bin/pseq.js",
    ...requiredBinaries.map((entry) => `bin/${entry.platformArch}/${entry.binary}`),
  ]);
  const optionalFiles = new Set(["LICENSE", "LICENSE.md"]);
  const packedFiles = new Set(files.map((file) => normalizePackagePath(file.path)));

  for (const expected of expectedFiles) {
    check(packedFiles.has(expected), `npm package is missing ${expected}`);
  }

  for (const file of files) {
    const path = normalizePackagePath(file.path);
    if (!expectedFiles.has(path) && !optionalFiles.has(path)) {
      errors.push(`npm package contains unexpected file: ${path}`);
    }
    if (path.startsWith("bin/") && file.size <= 0) {
      errors.push(`npm package binary entry is empty: ${path}`);
    }
  }
}

function checkExecutable(path, label) {
  if (process.platform === "win32") {
    return;
  }
  const mode = statSync(path).mode;
  check((mode & 0o111) !== 0, `${label} must have an executable mode`);
}

function cargoPackageField(content, field) {
  let inPackageSection = false;
  for (const line of content.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed === "[package]") {
      inPackageSection = true;
      continue;
    }
    if (inPackageSection && trimmed.startsWith("[")) {
      break;
    }
    if (!inPackageSection) {
      continue;
    }

    const match = trimmed.match(new RegExp(`^${field}\\s*=\\s*"([^"]+)"`));
    if (match) {
      return match[1];
    }
  }
  return undefined;
}

function cargoLockPackageField(content, packageName, field) {
  let inPackageSection = false;
  let currentPackage = {};

  for (const line of [...content.split(/\r?\n/), "[[package]]"]) {
    const trimmed = line.trim();
    if (trimmed === "[[package]]") {
      if (inPackageSection && currentPackage.name === packageName) {
        return currentPackage[field];
      }
      inPackageSection = true;
      currentPackage = {};
      continue;
    }

    if (!inPackageSection) {
      continue;
    }

    const match = trimmed.match(/^([A-Za-z0-9_-]+)\s*=\s*"([^"]*)"/);
    if (match) {
      currentPackage[match[1]] = match[2];
    }
  }

  return undefined;
}

function versionFromTag(rawTag) {
  const tag = rawTag.replace(/^refs\/tags\//, "");
  const match = tag.match(/^v(.+)$/);
  if (!match) {
    errors.push(`release tag must use v<version> format, got ${rawTag}`);
    return undefined;
  }
  const version = match[1];
  if (!isReleaseVersion(version)) {
    errors.push(`release tag version is not supported: ${rawTag}`);
    return undefined;
  }
  return version;
}

function isReleaseVersion(version) {
  return /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version);
}

function readJson(path) {
  try {
    return JSON.parse(readText(path));
  } catch (error) {
    errors.push(`failed to parse JSON ${path}: ${error.message}`);
    return {};
  }
}

function readText(path) {
  try {
    return readFileSync(path, "utf8");
  } catch (error) {
    errors.push(`failed to read ${path}: ${error.message}`);
    return "";
  }
}

function check(condition, message) {
  if (!condition) {
    errors.push(message);
  }
}

function normalizePackagePath(path) {
  return path.replaceAll("\\", "/");
}

function displayPackagePath(path) {
  return normalizePackagePath(path.slice(packageRoot.length + 1));
}

function parseArgs(args) {
  const parsed = {
    quiet: false,
    skipBinaries: false,
    skipPack: false,
  };
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--quiet") {
      parsed.quiet = true;
    } else if (arg === "--skip-binaries") {
      parsed.skipBinaries = true;
    } else if (arg === "--skip-pack") {
      parsed.skipPack = true;
    } else if (arg === "--tag") {
      parsed.tag = requireValue(args, ++index, arg);
    } else if (arg === "--version") {
      parsed.version = requireValue(args, ++index, arg);
    } else if (arg === "--package-root") {
      parsed.packageRoot = requireValue(args, ++index, arg);
    } else if (arg === "--repo-root") {
      parsed.repoRoot = requireValue(args, ++index, arg);
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
