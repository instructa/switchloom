#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(scriptDirectory, "..", "..");

function loadForbiddenPatterns() {
  const patternsFile = join(projectRoot, ".forbidden-paths.regex");
  if (!existsSync(patternsFile)) return [];
  return readFileSync(patternsFile, "utf8")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"))
    .map((pattern) => new RegExp(pattern));
}

function gitFiles() {
  const args = process.argv.includes("--tracked")
    ? ["ls-files"]
    : ["diff", "--cached", "--name-only", "--diff-filter=ACMR"];
  const output = execFileSync("git", args, { cwd: projectRoot, encoding: "utf8" });
  return output.trim().split("\n").filter(Boolean);
}

const patterns = loadForbiddenPatterns();
const forbidden = gitFiles().flatMap((file) => {
  const pattern = patterns.find((candidate) => candidate.test(file));
  return pattern ? [{ file, pattern: pattern.source }] : [];
});

if (forbidden.length > 0) {
  console.error("Forbidden files detected:");
  for (const { file, pattern } of forbidden) {
    console.error(`- ${file} (pattern: ${pattern})`);
  }
  process.exit(1);
}

console.log("No forbidden files detected");
