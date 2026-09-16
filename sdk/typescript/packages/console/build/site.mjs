import { copyFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const packageDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const site = path.join(packageDirectory, "dist/site");
const shell = path.join(packageDirectory, "src/index.html");

const directory = await stat(site).catch(() => undefined);
if (directory === undefined || !directory.isDirectory()) {
  throw new Error(`browser output directory is missing: ${site}; run tsc -p tsconfig.browser.json first`);
}
await copyFile(shell, path.join(site, "index.html"));
