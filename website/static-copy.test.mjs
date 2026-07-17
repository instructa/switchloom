import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { startupMessage } from "./serve.mjs";

const websiteRoot = dirname(fileURLToPath(import.meta.url));

test("status legend headline matches the six published state labels", async () => {
  const html = await readFile(join(websiteRoot, "index.html"), "utf8");
  assert.match(html, /<h2 id="legend-title">Six states, no ambiguity<\/h2>/);

  const legend = html.match(/<section class="legend"[\s\S]*?<\/section>/)?.[0] ?? "";
  const labels = [...legend.matchAll(/<dt>([^<]+)<\/dt>/g)].map((match) => match[1]);
  assert.deepEqual(labels, ["Official", "Signed", "Recommended", "Experimental", "Custom", "Unverified"]);
});

test("startup message is branded as Switchloom", () => {
  assert.equal(startupMessage(4173), "Switchloom catalog listening on http://127.0.0.1:4173");
});
