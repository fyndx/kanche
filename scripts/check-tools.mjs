// SPDX-License-Identifier: MPL-2.0

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const root = new URL("../", import.meta.url);
const expected = JSON.parse(readFileSync(new URL("package.json", root)));
if (process.versions.node !== expected.engines.node) {
  throw new Error(
    `Use Node ${expected.engines.node}; run nvm install && nvm use at the repository root.`,
  );
}
const npm = execFileSync("npm", ["--version"], { encoding: "utf8" }).trim();
if (npm !== expected.engines.npm) {
  throw new Error(
    `Use npm ${expected.engines.npm}; run npm install --global npm@${expected.engines.npm}.`,
  );
}
console.log(`Tool versions OK: Node ${process.versions.node}, npm ${npm}`);
