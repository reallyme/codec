// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

const bundledModuleUrl = new URL("../dist/wasm/reallyme_codec_wasm.js", import.meta.url).href;
const fixtureModuleUrl = new URL("./wasm-module-fixture.mjs", import.meta.url).href;

export const resolve = async (specifier, context, nextResolve) => {
  const resolved = await nextResolve(specifier, context);
  if (resolved.url === bundledModuleUrl) {
    return { ...resolved, url: fixtureModuleUrl, shortCircuit: true };
  }
  return resolved;
};
