#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { copyFileSync, mkdirSync, statSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { arch, platform } from "node:process";

const usage =
  "usage: node scripts/stage_kotlin_native_resource.mjs <native-library> <resources-root> [platform-arch]";

const fail = (message) => {
  console.error(`stage kotlin native resource failed: ${message}`);
  process.exit(1);
};

const [, , libraryPathArg, resourcesRootArg, platformArchArg] = process.argv;
if (libraryPathArg === undefined || resourcesRootArg === undefined) {
  fail(usage);
}

const normalizePlatform = (value) => {
  switch (value) {
    case "darwin":
    case "macos":
      return "macos";
    case "linux":
      return "linux";
    case "win32":
    case "windows":
      return "windows";
    default:
      fail(`unsupported platform ${value}`);
  }
};

const normalizeArch = (value) => {
  switch (value) {
    case "arm64":
    case "aarch64":
      return "aarch64";
    case "x64":
    case "amd64":
    case "x86_64":
      return "x86_64";
    default:
      fail(`unsupported architecture ${value}`);
  }
};

const expectedFileName = (normalizedPlatform) => {
  switch (normalizedPlatform) {
    case "macos":
      return "libreallyme_codec_ffi.dylib";
    case "windows":
      return "reallyme_codec_ffi.dll";
    case "linux":
      return "libreallyme_codec_ffi.so";
    default:
      fail(`unsupported platform ${normalizedPlatform}`);
  }
};

const [platformPart, archPart] =
  platformArchArg === undefined ? [platform, arch] : platformArchArg.split("-");
if (platformPart === undefined || archPart === undefined) {
  fail("platform-arch must look like linux-x86_64, macos-aarch64, or windows-x86_64");
}

const normalizedPlatform = normalizePlatform(platformPart);
const normalizedArch = normalizeArch(archPart);
const expectedName = expectedFileName(normalizedPlatform);
const source = resolve(libraryPathArg);
const resourcesRoot = resolve(resourcesRootArg);

try {
  if (!statSync(source).isFile()) {
    fail(`${source} is not a file`);
  }
} catch {
  fail(`${source} does not exist`);
}

if (basename(source) !== expectedName) {
  fail(`${source} must be named ${expectedName} for ${normalizedPlatform}-${normalizedArch}`);
}

const destinationDirectory = join(
  resourcesRoot,
  "me",
  "really",
  "codec",
  "native",
  `${normalizedPlatform}-${normalizedArch}`,
);
mkdirSync(destinationDirectory, { recursive: true });
copyFileSync(source, join(destinationDirectory, expectedName));
