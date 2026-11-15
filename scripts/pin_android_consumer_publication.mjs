// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { readFile, readdir, writeFile } from "node:fs/promises";
import { isAbsolute, join } from "node:path";

const COORDINATE = "me/really/codec-android";
const STABLE_VERSION = /^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$/u;

const pinPublication = async (repository, verificationPath) => {
  const modulePath = join(repository, COORDINATE);
  const entries = await readdir(modulePath, { withFileTypes: true });
  const versions = entries.filter((entry) => entry.isDirectory());
  if (versions.length !== 1 || !STABLE_VERSION.test(versions[0].name)) {
    return "expected one staged codec-android version";
  }
  const version = versions[0].name;
  const metadata = await readFile(verificationPath, "utf8");
  const components = "   <components>\n";
  if (metadata.split(components).length !== 2 ||
      /<component\s+group="me\.really"\s+name="codec-android"/u.test(metadata)) {
    return "unexpected consumer verification metadata";
  }

  // The consumer resolves a local snapshot produced in the CI job. Keep
  // strict verification and pin each file while retaining all third-party pins.
  const artifacts = [];
  for (const extension of ["aar", "module", "pom"]) {
    const file = `codec-android-${version}.${extension}`;
    const hash = createHash("sha256");
    for await (const bytes of createReadStream(join(modulePath, version, file))) {
      hash.update(bytes);
    }
    artifacts.push(
      `         <artifact name="${file}">\n` +
      `            <sha256 value="${hash.digest("hex")}"/>\n` +
      "         </artifact>\n",
    );
  }
  const entry = `      <component group="me.really" name="codec-android" version="${version}">\n` +
    artifacts.join("") + "      </component>\n";
  await writeFile(verificationPath, metadata.replace(components, components + entry));
  return null;
};

const [repository, verificationPath] = process.argv.slice(2);
let failure;
try {
  failure = process.argv.length === 4 && isAbsolute(repository) && isAbsolute(verificationPath)
    ? await pinPublication(repository, verificationPath)
    : "expected absolute publication and verification metadata paths";
} catch {
  failure = "could not read the publication or write its verification metadata";
}
if (failure !== null) {
  console.error(`Android consumer pinning failed: ${failure}`);
  process.exitCode = 1;
}
