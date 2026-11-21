// SPDX-License-Identifier: MPL-2.0
import SwaggerParser from "@apidevtools/swagger-parser";
for (const file of [
  "docs/contracts/app-owned.openapi.json",
  "docs/contracts/hosted.openapi.json",
]) {
  await SwaggerParser.validate(file);
}
console.log("OpenAPI 3.1 contract validated");
