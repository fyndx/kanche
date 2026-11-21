// SPDX-License-Identifier: MPL-2.0
import SwaggerParser from "@apidevtools/swagger-parser";
await SwaggerParser.validate("docs/contracts/app-owned.openapi.json");
console.log("OpenAPI 3.1 contract validated");
