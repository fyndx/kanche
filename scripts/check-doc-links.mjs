// SPDX-License-Identifier: MPL-2.0

// Check repository-relative Markdown targets without visiting external services.
import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

function walk(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (["node_modules", "target", ".git", ".tools"].includes(entry.name))
      return [];
    const path = resolve(dir, entry.name);
    return entry.isDirectory()
      ? walk(path)
      : path.endsWith(".md")
        ? [path]
        : [];
  });
}
let checked = 0;
for (const file of walk(".")) {
  const source = readFileSync(file, "utf8");
  for (const match of source.matchAll(/\]\(([^\s)]+)(?:\s+"[^"]*")?\)/g)) {
    const target = match[1].replace(/^<|>$/g, "");
    if (/^(?:[a-z][a-z0-9+.-]*:|#)/i.test(target)) continue;
    const path = decodeURIComponent(target.split("#")[0]);
    assert.ok(
      existsSync(resolve(dirname(file), path)),
      `Broken relative link in ${file}: ${target}`,
    );
    checked++;
  }
}
console.log(
  `Checked ${checked} relative Markdown file links (anchors and remote URLs are not checked).`,
);
