#!/usr/bin/env node
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { requiredBinaries, requiredBinaryPath } from "./platforms.js";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(here, "..");

const missing = requiredBinaries
  .map((entry) => requiredBinaryPath(packageRoot, entry))
  .filter((path) => !existsSync(path));

if (missing.length > 0) {
  console.error("missing pseq release binaries:");
  for (const path of missing) {
    console.error(`- ${path}`);
  }
  process.exit(1);
}
