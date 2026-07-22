#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const projectRoot = resolve(dirname(scriptPath), "..", "..");
const maximumTextFileBytes = 2 * 1024 * 1024;

const safeHomeNames = new Set(["<user>", "example", "runner", "test", "tester", "user"]);
const safeEmailDomains = new Set(["example.com", "example.org", "users.noreply.github.com"]);

function git(args) {
  return execFileSync("git", args, {
    cwd: projectRoot,
    encoding: "utf8",
    maxBuffer: 32 * 1024 * 1024,
  });
}

function isSafeHomePath(value) {
  const match = value.match(/^\/(?:Users|home)\/([^/]+)/);
  return match ? safeHomeNames.has(match[1]) : true;
}

function isSafeEmail(value) {
  const domain = value.split("@").at(-1)?.toLowerCase();
  return domain ? safeEmailDomains.has(domain) : false;
}

export function scanLine(content, file, line) {
  const findings = [];
  const homePattern = /\/(?:Users|home)\/[A-Za-z0-9._<>-]+/g;
  const emailPattern = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g;
  const callbackPattern = /https?:\/\/[^\s"'<>]*(?:deviceauth|oauth[^\s"'<>]*callback)[^\s"'<>]*(?:code|state|token)=[A-Za-z0-9._~-]{8,}/gi;

  for (const match of content.matchAll(homePattern)) {
    if (!isSafeHomePath(match[0])) findings.push({ category: "personal-home-path", file, line });
  }
  for (const match of content.matchAll(emailPattern)) {
    if (!isSafeEmail(match[0])) findings.push({ category: "personal-email", file, line });
  }
  if (callbackPattern.test(content)) {
    findings.push({ category: "credential-bearing-auth-url", file, line });
  }

  return findings;
}

export function scanUnifiedDiff(diff) {
  const findings = [];
  let file = null;
  let newLine = 0;

  for (const rawLine of diff.split("\n")) {
    if (rawLine.startsWith("+++ b/")) {
      file = rawLine.slice(6);
      continue;
    }
    const hunk = rawLine.match(/^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
    if (hunk) {
      newLine = Number.parseInt(hunk[1], 10);
      continue;
    }
    if (!file || rawLine.startsWith("--- ") || rawLine.startsWith("diff --git ")) continue;
    if (rawLine.startsWith("+")) {
      findings.push(...scanLine(rawLine.slice(1), file, newLine));
      newLine += 1;
    } else if (!rawLine.startsWith("-")) {
      newLine += 1;
    }
  }

  return findings;
}

function scanUntrackedFiles() {
  const output = execFileSync("git", ["ls-files", "--others", "--exclude-standard", "-z"], {
    cwd: projectRoot,
    encoding: "buffer",
  });
  const findings = [];
  for (const file of output.toString("utf8").split("\0").filter(Boolean)) {
    const absolutePath = resolve(projectRoot, file);
    if (!existsSync(absolutePath) || statSync(absolutePath).size > maximumTextFileBytes) continue;
    const content = readFileSync(absolutePath);
    if (content.includes(0)) continue;
    content
      .toString("utf8")
      .split("\n")
      .forEach((lineContent, index) => findings.push(...scanLine(lineContent, file, index + 1)));
  }
  return findings;
}

function uniqueFindings(findings) {
  return [...new Map(findings.map((finding) => [
    `${finding.category}:${finding.file}:${finding.line}`,
    finding,
  ])).values()];
}

function collectFindings(args) {
  const baseIndex = args.indexOf("--base");
  if (baseIndex !== -1) {
    const base = args[baseIndex + 1];
    if (!base) throw new Error("--base requires a Git revision");
    return scanUnifiedDiff(git(["diff", "--no-ext-diff", "--no-color", "--unified=0", base, "--"]));
  }
  if (args.includes("--worktree")) {
    return uniqueFindings([
      ...scanUnifiedDiff(git(["diff", "--no-ext-diff", "--no-color", "--unified=0", "--"])),
      ...scanUnifiedDiff(git(["diff", "--cached", "--no-ext-diff", "--no-color", "--unified=0", "--"])),
      ...scanUntrackedFiles(),
    ]);
  }
  return scanUnifiedDiff(git(["diff", "--cached", "--no-ext-diff", "--no-color", "--unified=0", "--"]));
}

function main() {
  const findings = uniqueFindings(collectFindings(process.argv.slice(2)));
  if (findings.length === 0) {
    console.log("No sensitive personal content detected in added lines");
    return;
  }

  console.error("Sensitive personal content detected in added lines:");
  for (const finding of findings) {
    console.error(`- ${finding.file}:${finding.line} (${finding.category})`);
  }
  console.error("Replace personal values with placeholders or keep the generated file outside Git.");
  process.exitCode = 1;
}

if (resolve(process.argv[1] ?? "") === scriptPath) main();
