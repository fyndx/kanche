// SPDX-License-Identifier: MPL-2.0

import { execFileSync } from "node:child_process";

const npm = execFileSync("npm", ["--version"], { encoding: "utf8" }).trim();
console.log(`Tool versions: Node ${process.versions.node}, npm ${npm}`);
