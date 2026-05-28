import { join } from "node:path";

export const requiredBinaries = [
  { platformArch: "darwin-arm64", binary: "pseq" },
  { platformArch: "darwin-x64", binary: "pseq" },
  { platformArch: "linux-arm64", binary: "pseq" },
  { platformArch: "linux-x64", binary: "pseq" },
  { platformArch: "win32-x64", binary: "pseq.exe" },
];

export function currentPlatformArch() {
  return `${process.platform}-${process.arch}`;
}

export function requiredBinaryFor(platformArch) {
  return requiredBinaries.find((entry) => entry.platformArch === platformArch);
}

export function requiredBinaryPath(packageRoot, entry) {
  return join(packageRoot, "bin", entry.platformArch, entry.binary);
}
