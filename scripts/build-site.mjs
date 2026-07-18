#!/usr/bin/env node
import { cp, mkdir, readdir, readFile, rm, stat } from "node:fs/promises";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

export const REQUIRED_SITE_FILES = Object.freeze(["_headers", "index.html", "data/catalog.json"]);

async function regularFile(path) {
  try {
    return (await stat(path)).isFile();
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }
}

export async function publicationFiles(root) {
  const files = [];

  async function visit(directory) {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) await visit(path);
      else if (entry.isFile()) files.push(relative(root, path).split(sep).join("/"));
      else throw new Error(`publish output contains unsupported entry: ${relative(root, path)}`);
    }
  }

  await visit(root);
  return files.sort();
}

export async function verifyPublication(root) {
  const actual = await publicationFiles(root);
  const missing = REQUIRED_SITE_FILES.filter((file) => !actual.includes(file));
  if (missing.length > 0) throw new Error(`publish output is missing: ${missing.join(", ")}`);
  if (!actual.some((file) => file.startsWith("_astro/") && file.endsWith(".js"))) {
    throw new Error("publish output is missing the Astro client bundle");
  }
  if (!actual.some((file) => file.startsWith("data/bundles/") && file.endsWith(".json"))) {
    throw new Error("publish output is missing canonical bundle downloads");
  }
  return actual;
}

export async function buildSite({ sourceRoot, outputRoot }) {
  const source = resolve(sourceRoot);
  const output = resolve(outputRoot);
  if (output === source || output.startsWith(`${source}${sep}`) || source.startsWith(`${output}${sep}`)) {
    throw new Error("catalog source and site output must be separate");
  }

  const catalogPath = join(source, "catalog.json");
  const bundlesPath = join(source, "bundles");
  if (!(await regularFile(catalogPath))) throw new Error("missing canonical catalog: catalog.json");
  const catalog = JSON.parse(await readFile(catalogPath, "utf8"));
  if (!Array.isArray(catalog.compositions) || catalog.compositions.length === 0) {
    throw new Error("canonical catalog has no compositions");
  }
  if ((await publicationFiles(bundlesPath)).length === 0) throw new Error("canonical bundle directory is empty");
  if (!(await regularFile(join(output, "index.html")))) throw new Error("Astro build must run before catalog publication");

  const destination = join(output, "data");
  await rm(destination, { recursive: true, force: true });
  await mkdir(destination, { recursive: true });
  await cp(catalogPath, join(destination, "catalog.json"));
  await cp(bundlesPath, join(destination, "bundles"), { recursive: true });

  return verifyPublication(output);
}

const modulePath = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  const repositoryRoot = resolve(dirname(modulePath), "..");
  buildSite({
    sourceRoot: join(repositoryRoot, "website", "data"),
    outputRoot: join(repositoryRoot, "dist", "website"),
  })
    .then((files) => console.log(`published ${files.length} Astro website artifacts in dist/website`))
    .catch((error) => {
      console.error(error instanceof Error ? error.message : error);
      process.exitCode = 1;
    });
}
