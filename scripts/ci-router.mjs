#!/usr/bin/env node
import { appendFileSync } from "node:fs";

const ALL = Object.freeze({ rust: true, website: true, distribution: true });
const NONE = Object.freeze({ rust: false, website: false, distribution: false });

const isDocumentation = (file) => !file.startsWith("website/") && (file === "README.md" || file === "LICENSE" || file.startsWith("docs/") || file.endsWith(".md"));
const isWebsite = (file) => file.startsWith("website/") || file === "astro.config.mjs" || file === "tailwind.config.mjs";
const isRust = (file) => file === "Cargo.toml" || file === "Cargo.lock" || file === "rust-toolchain.toml" || file === "rustfmt.toml" || file === "clippy.toml" || file.startsWith("src/") || file.startsWith("crates/") || file.startsWith("xtask/");
const isDistribution = (file) => file === "package.json" || file === "package-lock.json" || file === "npm-shrinkwrap.json" || file.startsWith("npm/");

/**
 * Convert changed repository-relative paths into the CI jobs they require.
 * Unknown paths intentionally select every job: this is the fail-closed policy.
 */
export function classifyPaths(paths) {
  const jobs = { ...NONE };
  const changed = [...paths].filter(Boolean);

  for (const file of changed) {
    if (file.startsWith(".github/workflows/")) return { ...ALL, reason: "workflow" };
    if (isDocumentation(file)) continue;
    if (isWebsite(file)) {
      jobs.website = true;
      continue;
    }
    if (isRust(file)) {
      jobs.rust = true;
      jobs.distribution = true;
      continue;
    }
    if (isDistribution(file)) {
      jobs.distribution = true;
      continue;
    }
    return { ...ALL, reason: "unknown" };
  }

  return { ...jobs, reason: changed.length === 0 || Object.values(jobs).every((selected) => !selected) ? "docs" : "paths" };
}

function usage() {
  return "Usage: node scripts/ci-router.mjs [--github-output FILE] <changed-path>...";
}

function main(args) {
  let outputFile;
  if (args[0] === "--github-output") {
    outputFile = args[1];
    args = args.slice(2);
    if (!outputFile) throw new Error("--github-output requires a file path");
  }
  if (args.includes("--help") || args.includes("-h")) {
    console.log(usage());
    return;
  }

  const result = classifyPaths(args);
  if (outputFile) {
    appendFileSync(outputFile, Object.entries(result).map(([key, value]) => `${key}=${value}`).join("\n") + "\n");
  }
  console.log(JSON.stringify(result));
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(`ci-router: ${error.message}`);
    process.exitCode = 1;
  }
}
