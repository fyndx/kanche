// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

// Exact SPDX expressions: new or compound licenses require explicit review.
const allowed = new Set([
  "MIT",
  "ISC",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "Apache-2.0",
  "0BSD",
  "CC0-1.0",
  "Unlicense",
  "CC-BY-4.0",
  "BlueOak-1.0.0",
  "Python-2.0",
  "PSF-2.0", // argparse (OpenAPI validator): PSF notice and changes-summary terms reviewed.
  "(MIT OR CC0-1.0)",
  "(MIT AND Zlib)",
  "(Apache-2.0 OR MIT)",
]);
for (const file of ["package-lock.json"]) {
  const lock = JSON.parse(readFileSync(file, "utf8"));
  for (const [name, pkg] of Object.entries(lock.packages)) {
    if (!name) {
      assert.equal(
        pkg.license,
        "MPL-2.0",
        `${file}: root package license must be MPL-2.0`,
      );
      continue;
    }
    if (pkg.link) {
      const workspace = lock.packages[pkg.resolved];
      assert.ok(
        workspace?.license === "MPL-2.0" &&
          JSON.parse(readFileSync(`${pkg.resolved}/package.json`, "utf8"))
            .private === true,
        `${file}: invalid internal workspace ${name}`,
      );
      continue;
    }
    if (name.startsWith("packages/")) {
      assert.ok(
        pkg.license === "MPL-2.0" &&
          JSON.parse(readFileSync(`${name}/package.json`, "utf8")).private ===
            true,
        `${file}: invalid internal package ${name}`,
      );
      continue;
    }
    assert.ok(
      allowed.has(pkg.license),
      `${file}: review license ${pkg.license ?? "MISSING"} for ${name}@${pkg.version}`,
    );
  }
  console.log(`Dependency licenses passed: ${file}`);
}
