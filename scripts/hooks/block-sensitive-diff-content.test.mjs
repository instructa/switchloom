import assert from "node:assert/strict";
import test from "node:test";

import { scanLine, scanUnifiedDiff } from "./block-sensitive-diff-content.mjs";

test("reports private values by category and location without returning the value", () => {
  const privateHome = ["", "Users", "private-name", "project"].join("/");
  const privateEmail = ["person", "private.invalid"].join("@");
  const findings = scanLine(
    `owner=${privateHome} contact=${privateEmail}`,
    "receipt.json",
    7,
  );

  assert.deepEqual(findings, [
    { category: "personal-home-path", file: "receipt.json", line: 7 },
    { category: "personal-email", file: "receipt.json", line: 7 },
  ]);
  assert.equal(JSON.stringify(findings).includes("private-name"), false);
  assert.equal(JSON.stringify(findings).includes("person@"), false);
});

test("allows generic paths and non-personal bot or example email addresses", () => {
  assert.deepEqual(scanLine("/Users/<user>/repo", "docs.md", 1), []);
  assert.deepEqual(scanLine("/home/runner/work", "workflow.yml", 2), []);
  assert.deepEqual(scanLine("release-bot@users.noreply.github.com", "workflow.yml", 3), []);
  assert.deepEqual(scanLine("user@example.com", "fixture.txt", 4), []);
});

test("detects credential-bearing authentication callback URLs", () => {
  const callback = [
    "https://auth.example.invalid/deviceauth/callback",
    "code=abcdefgh1234",
  ].join("?");
  assert.deepEqual(
    scanLine(callback, "session.log", 9),
    [{ category: "credential-bearing-auth-url", file: "session.log", line: 9 }],
  );
});

test("maps added diff lines to their destination line numbers", () => {
  const privateHome = ["", "Users", "private-name", "work"].join("/");
  const findings = scanUnifiedDiff(`diff --git a/new.md b/new.md
--- a/new.md
+++ b/new.md
@@ -2,0 +3,2 @@
+safe
+path=${privateHome}
`);

  assert.deepEqual(findings, [
    { category: "personal-home-path", file: "new.md", line: 4 },
  ]);
});
