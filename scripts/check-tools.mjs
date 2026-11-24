// SPDX-License-Identifier: MPL-2.0

import { execFileSync } from "node:child_process";
if (!process.release.lts) {
  throw new Error(
    `Node ${process.versions.node} is not an LTS build. Install the latest Node.js LTS with your preferred tool manager; see README.md#development-setup.`,
  );
}
const npm = execFileSync("npm", ["--version"], { encoding: "utf8" }).trim();
console.log(
  `Tool versions: Node ${process.versions.node} (${process.release.lts} LTS), npm ${npm}`,
);
