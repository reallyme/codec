#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set +x
set -euo pipefail
umask 077

fail() {
  printf 'Maven Central bundle failed: %s\n' "$1" >&2
  exit 1
}

[[ "$#" -eq 2 ]] || fail "usage: ./scripts/maven-central-bundle.local.sh VERSION SOURCE_SHA"
readonly version="$1" source_sha="$2"
[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] ||
  fail "select a stable release version"
[[ "$source_sha" =~ ^[a-f0-9]{40}$ ]] || fail "select a full source SHA from Maven preflight"

# Keep the passphrase out of child environments. Only GPG receives it on stdin.
export -n signing_key signing_passphrase
readonly signing_key="${MAVEN_SIGNING_KEY_ID:-}" signing_passphrase="${MAVEN_SIGNING_PASSWORD:-}"
unset MAVEN_SIGNING_KEY_ID MAVEN_SIGNING_KEY MAVEN_SIGNING_PASSWORD
[[ "$signing_key" =~ ^([[:xdigit:]]{16}|[[:xdigit:]]{40}|[[:xdigit:]]{64})$ ]] ||
  fail "set MAVEN_SIGNING_KEY_ID to a GPG long key ID or fingerprint"
[[ -n "$signing_passphrase" ]] || fail "load MAVEN_SIGNING_PASSWORD before signing"

for tool in git gpg node zip; do
  command -v "$tool" >/dev/null || fail "$tool is required"
done

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
readonly adapter=scripts/stage-maven-central-bundle.sh
cd -- "$repo_root"
if [[ "$(git rev-parse --verify HEAD)" != "$source_sha" ]] || ! git diff --quiet --no-ext-diff HEAD --; then
  fail "check out the selected source SHA with unchanged tracked files"
fi
if ! git ls-files --error-unmatch -- "$adapter" >/dev/null 2>&1 ||
   [[ ! -f "$adapter" || ! -x "$adapter" || -L "$adapter" ]]; then
  fail "provide the tracked, executable $adapter"
fi

output_dir="${MAVEN_CENTRAL_OUTPUT_DIR:-"$repo_root/build/maven-central-upload/out"}"
[[ "$output_dir" == /* ]] || fail "MAVEN_CENTRAL_OUTPUT_DIR must be absolute"
mkdir -p -- "$output_dir"
output_dir="$(cd -- "$output_dir" && pwd -P)"
[[ "$output_dir" != / && "$output_dir" != "$repo_root" ]] ||
  fail "select a dedicated output directory for the Maven bundles"

# Build and sign in private staging on the same filesystem as the final ZIP.
# Only this invocation's staging may be removed.
scratch="$(mktemp -d "$output_dir/.maven-bundle.XXXXXXXX")"
trap 'rm -rf -- "$scratch"' EXIT
readonly repository="$scratch/repository" payload="$scratch/files"
mkdir -- "$repository"

# The repository adapter verifies the selected preflight and stages the
# unsigned Maven files for this version and SHA.
bash -e -o pipefail "$adapter" "$version" "$source_sha" "$repository" >&2
[[ -d "$repository" && ! -L "$repository" ]] || fail "the adapter must preserve the staging directory"
special="$(find "$repository" -mindepth 1 ! -type f ! -type d -print -quit)"
[[ -z "$special" ]] || fail "the staging tree must contain only regular files and directories"
find "$repository" -type f -print0 >"$payload"
[[ -s "$payload" ]] || fail "the adapter did not stage a Maven publication"

has_pom=false
while IFS= read -r -d '' artifact; do
  path="${artifact#"$repository/"}"
  if [[ ! "$path" =~ ^([a-zA-Z0-9_-][a-zA-Z0-9_.-]*/){3,}[a-zA-Z0-9_-][a-zA-Z0-9_.-]*$ ||
        "${artifact%/*}" != */"$version" ]]; then
    fail "every staged file must belong to the selected version in Maven repository layout"
  fi
  case "$path" in
    */maven-metadata.xml*|*.asc|*.md5|*.sha1|*.sha256|*.sha512)
      fail "stage unsigned publication files without local repository metadata or checksum sidecars" ;;
    *.pom) has_pom=true ;;
  esac
done <"$payload"
[[ "$has_pom" == true ]] || fail "the staged publication must include a POM"

while IFS= read -r -d '' artifact; do
  gpg --batch --no-tty --pinentry-mode loopback --passphrase-fd 0 \
    --local-user "$signing_key" --armor --detach-sign --output "$artifact.asc" "$artifact" \
    <<<"$signing_passphrase" || fail "could not sign a staged Maven file"
done <"$payload"

while IFS= read -r -d '' artifact; do
  gpg --batch --no-tty --no-auto-key-retrieve --verify "$artifact.asc" "$artifact" ||
    fail "a Maven bundle signature failed verification"
done <"$payload"

node --input-type=module - "$payload" <<'NODE'
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { readFile, writeFile } from "node:fs/promises";

try {
  const entries = (await readFile(process.argv[2], "utf8")).split("\0").filter(Boolean);
  for (const entry of entries) {
    for (const path of [entry, `${entry}.asc`]) {
      const hashes = new Map(["md5", "sha1", "sha256", "sha512"].map(
        (algorithm) => [algorithm, createHash(algorithm)],
      ));
      for await (const chunk of createReadStream(path)) {
        for (const hash of hashes.values()) hash.update(chunk);
      }
      for (const [algorithm, hash] of hashes) {
        await writeFile(`${path}.${algorithm}`, `${hash.digest("hex")}\n`, { flag: "wx" });
      }
    }
  }
} catch {
  console.error("Maven Central bundle failed: could not write publication checksums");
  process.exitCode = 1;
}
NODE

archive="$scratch/bundle.zip"
(cd -- "$repository" && COPYFILE_DISABLE=1 zip -X -q -r "$archive" .) ||
  fail "could not create the Maven Central ZIP"
[[ -s "$archive" ]] || fail "the completed Maven Central ZIP is empty"

# A hard link publishes a complete archive and fails if its destination
# exists. Concurrent runs can choose different filenames without clobbering.
node --input-type=module - "$archive" "$output_dir/reallyme-maven-central-$version" <<'NODE'
import { linkSync } from "node:fs";

const [archive, stem] = process.argv.slice(2);
const maxNames = 1000;
let saved = false;
for (let attempt = 1; attempt <= maxNames; attempt += 1) {
  const destination = attempt === 1 ? `${stem}.zip` : `${stem}-${attempt}.zip`;
  try {
    linkSync(archive, destination);
    process.stdout.write(`${destination}\n`);
    saved = true;
    break;
  } catch (error) {
    if (error?.code !== "EEXIST") break;
  }
}
if (!saved) {
  console.error("Maven Central bundle failed: could not save the completed ZIP without overwriting an existing file");
  process.exitCode = 1;
}
NODE
