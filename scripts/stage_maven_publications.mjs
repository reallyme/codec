// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
// SPDX-License-Identifier: MIT OR Apache-2.0

import { constants, copyFileSync, lstatSync, mkdirSync, realpathSync } from "node:fs";
import { dirname, isAbsolute, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

export class MavenPublicationError extends Error {}

export function publicationLayout(version, sourceSha) {
  if (!/^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$/u.test(version) ||
      !/^[0-9a-f]{40}$/u.test(sourceSha)) {
    throw new MavenPublicationError("expected a stable version and full source SHA");
  }
  return [
    { kind: "jvm", artifact: "codec", prefix: "", extensions: [".jar", ".pom", ".module", "-sources.jar", "-javadoc.jar"] },
    { kind: "android", artifact: "codec-android", prefix: "repository/", extensions: [".aar", ".pom", ".module", "-sources.jar"] },
  ].map(({ kind, artifact, prefix, extensions }) => ({
    name: `maven-${kind}-${version}-${sourceSha}`,
    files: extensions.map((extension) => {
      const target = `me/really/${artifact}/${version}/${artifact}-${version}${extension}`;
      return { source: `${prefix}${target}`, target };
    }),
  }));
}

export function stageMavenPublications(packages, version, sourceSha, destination) {
  if (!isAbsolute(packages) || !isAbsolute(destination) || !lstatSync(destination).isDirectory()) {
    throw new MavenPublicationError("expected an absolute package directory and a private staging directory");
  }
  for (const { name, files } of publicationLayout(version, sourceSha)) {
    const packageRoot = join(packages, name);
    const canonicalRoot = realpathSync(packageRoot);
    for (const { source, target } of files) {
      const input = join(packageRoot, source);
      const status = lstatSync(input);
      if (!status.isFile() || status.size === 0 || relative(canonicalRoot, realpathSync(input)) !== source) {
        throw new MavenPublicationError("invalid versioned publication file");
      }
      const output = join(destination, target);
      mkdirSync(dirname(output), { recursive: true });
      copyFileSync(input, output, constants.COPYFILE_EXCL);
    }
  }
}

const isMain = process.argv[1] !== undefined && fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  try {
    const [, , packages, version, sourceSha, destination, ...extra] = process.argv;
    if (destination === undefined || extra.length !== 0) {
      throw new MavenPublicationError("expected a package directory, version, source SHA, and staging directory");
    }
    stageMavenPublications(packages, version, sourceSha, destination);
  } catch (error) {
    const message = error instanceof MavenPublicationError ? error.message : "missing or inaccessible versioned publication file";
    console.error(`Maven staging failed: ${message}`);
    process.exitCode = 1;
  }
}
