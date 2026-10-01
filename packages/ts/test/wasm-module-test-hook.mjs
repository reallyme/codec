// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { register } from "node:module";

// Redirect only the bundled module in this isolated test worker. The public
// installer still checks module identity; the fixture exercises response
// validation without adding a provider-injection path to the shipped package.
register("./wasm-module-test-loader.mjs", import.meta.url);
