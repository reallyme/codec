#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail
IFS=$'\n\t'

# Usage:
#   MAVEN_SIGNING_KEY_ID=<long-gpg-key-id-or-fingerprint> \
#   MAVEN_SIGNING_PASSWORD="..." \
#   KOTLIN_NATIVE_RESOURCES_DIR=/path/to/full/kotlin-native-resources \
#   ANDROID_NDK_HOME=/path/to/android-ndk \
#   ./scripts/maven_central_bundle_local.sh
#
# Output:
#   build/maven-central-upload/out/reallyme-maven-central-<version>.zip
#
# Upload the printed zip in Central Portal as a deployment bundle. The bundle is
# assembled in Maven repository layout and includes the JVM jar and Android AAR.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="${MAVEN_CENTRAL_WORK_DIR:-${ROOT_DIR}/build/maven-central-upload}"
BUNDLE_ROOT="${WORK_DIR}/bundle-root"
OUTPUT_DIR="${MAVEN_CENTRAL_OUTPUT_DIR:-${WORK_DIR}/out}"
GRADLE="${ROOT_DIR}/packages/kotlin/gradlew"
KOTLIN_NATIVE_RESOURCES_DIR="${KOTLIN_NATIVE_RESOURCES_DIR:-${ROOT_DIR}/build/kotlin-native-resources}"
ANDROID_JNI_LIBS_DIR_WAS_SET="${ANDROID_JNI_LIBS_DIR+x}"
ANDROID_JNI_LIBS_DIR="${ANDROID_JNI_LIBS_DIR:-${WORK_DIR}/android-jniLibs}"
ANDROID_NATIVE_ASSETS_DIR="${ANDROID_NATIVE_ASSETS_DIR:-${WORK_DIR}/android-native-assets}"
ANDROID_NDK_VERSION="${ANDROID_NDK_VERSION:-29.0.14206865}"
NATIVE_RESOURCE_WORKFLOW="${MAVEN_NATIVE_RESOURCE_WORKFLOW:-kotlin-android-package-preflight.yml}"
NATIVE_RESOURCE_ARTIFACT_PATTERN="${MAVEN_NATIVE_RESOURCE_ARTIFACT_PATTERN:-kotlin-native-*}"
NATIVE_RESOURCE_DOWNLOAD_DIR="${MAVEN_NATIVE_RESOURCE_DOWNLOAD_DIR:-${WORK_DIR}/kotlin-native-artifacts}"
NATIVE_RESOURCE_WORKFLOW_TIMEOUT_SECONDS="${MAVEN_NATIVE_RESOURCE_WORKFLOW_TIMEOUT_SECONDS:-3600}"

fail() {
  printf 'maven central bundle failed: %s\n' "$1" >&2
  exit 1
}

info() {
  printf '%s\n' "$1" >&2
}

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "required tool not found: $1"
  fi
}

read_gradle_version() {
  local file="$1"
  sed -n 's/^version = "\([^"]*\)".*/\1/p' "$file" | head -n 1
}

require_file() {
  if [ ! -f "$1" ]; then
    fail "missing required file: $1"
  fi
}

require_dir() {
  if [ ! -d "$1" ]; then
    fail "missing required directory: $1"
  fi
}

require_zip_entry() {
  local archive="$1"
  local entry="$2"
  if ! jar tf "$archive" | grep -Fx -- "$entry" >/dev/null; then
    fail "$archive is missing archive entry: $entry"
  fi
}

kotlin_native_resource_files() {
  printf '%s\n' \
    "${KOTLIN_NATIVE_RESOURCES_DIR}/me/really/codec/native/linux-x86_64/libreallyme_codec_ffi.so" \
    "${KOTLIN_NATIVE_RESOURCES_DIR}/me/really/codec/native/linux-aarch64/libreallyme_codec_ffi.so" \
    "${KOTLIN_NATIVE_RESOURCES_DIR}/me/really/codec/native/macos-x86_64/libreallyme_codec_ffi.dylib" \
    "${KOTLIN_NATIVE_RESOURCES_DIR}/me/really/codec/native/macos-aarch64/libreallyme_codec_ffi.dylib" \
    "${KOTLIN_NATIVE_RESOURCES_DIR}/me/really/codec/native/windows-x86_64/reallyme_codec_ffi.dll"
}

kotlin_native_resources_are_complete() {
  local native_file
  while IFS= read -r native_file; do
    if [ ! -f "$native_file" ]; then
      return 1
    fi
  done < <(kotlin_native_resource_files)
  return 0
}

current_git_ref() {
  local branch
  branch="$(git -C "$ROOT_DIR" rev-parse --abbrev-ref HEAD)"
  if [ "$branch" = "HEAD" ]; then
    git -C "$ROOT_DIR" rev-parse HEAD
  else
    printf '%s\n' "$branch"
  fi
}

find_successful_native_resource_run() {
  local head_sha="$1"
  gh run list \
    --workflow "$NATIVE_RESOURCE_WORKFLOW" \
    --commit "$head_sha" \
    --status success \
    --limit 20 \
    --json databaseId,headSha \
    --jq ".[] | select(.headSha == \"${head_sha}\") | .databaseId" \
    | head -n 1
}

latest_native_resource_run_for_head() {
  local head_sha="$1"
  gh run list \
    --workflow "$NATIVE_RESOURCE_WORKFLOW" \
    --commit "$head_sha" \
    --limit 20 \
    --json databaseId,headSha \
    --jq ".[] | select(.headSha == \"${head_sha}\") | .databaseId" \
    | head -n 1
}

native_resource_run_status() {
  local run_id="$1"
  gh run view "$run_id" \
    --json status,conclusion \
    --jq '[.status, (.conclusion // "")] | @tsv'
}

wait_for_native_resource_run_id() {
  local head_sha="$1"
  local excluded_run_id="${2:-}"
  local deadline
  local run_id
  deadline=$((SECONDS + NATIVE_RESOURCE_WORKFLOW_TIMEOUT_SECONDS))

  while [ "$SECONDS" -lt "$deadline" ]; do
    run_id="$(latest_native_resource_run_for_head "$head_sha")"
    if [ -n "$run_id" ] && [ "$run_id" != "$excluded_run_id" ]; then
      printf '%s\n' "$run_id"
      return
    fi
    sleep 5
  done

  fail "timed out waiting for ${NATIVE_RESOURCE_WORKFLOW} to start for ${head_sha}"
}

download_kotlin_native_resources_from_run() {
  local run_id="$1"
  local artifact_dir

  rm -rf "$NATIVE_RESOURCE_DOWNLOAD_DIR"
  mkdir -p "$NATIVE_RESOURCE_DOWNLOAD_DIR" "$KOTLIN_NATIVE_RESOURCES_DIR"
  info "Downloading JVM native resource artifacts from GitHub Actions run ${run_id}"
  gh run download "$run_id" \
    --pattern "$NATIVE_RESOURCE_ARTIFACT_PATTERN" \
    --dir "$NATIVE_RESOURCE_DOWNLOAD_DIR"

  while IFS= read -r artifact_dir; do
    cp -R "${artifact_dir}/." "$KOTLIN_NATIVE_RESOURCES_DIR/"
  done < <(find "$NATIVE_RESOURCE_DOWNLOAD_DIR" -mindepth 1 -maxdepth 1 -type d | sort)
}

ensure_kotlin_native_resources() {
  local head_sha
  local git_ref
  local run_id
  local run_state
  local run_status
  local run_conclusion
  local stale_run_id=""

  mkdir -p "$KOTLIN_NATIVE_RESOURCES_DIR"
  if kotlin_native_resources_are_complete; then
    return
  fi

  info "JVM native resources are incomplete; staging the current host library"
  "${ROOT_DIR}/scripts/build_kotlin_native_resource.sh" "$KOTLIN_NATIVE_RESOURCES_DIR"
  if kotlin_native_resources_are_complete; then
    return
  fi

  require_tool git
  require_tool gh
  head_sha="$(git -C "$ROOT_DIR" rev-parse HEAD)"
  git_ref="$(current_git_ref)"
  run_id="$(find_successful_native_resource_run "$head_sha")"
  if [ -z "$run_id" ]; then
    run_id="$(latest_native_resource_run_for_head "$head_sha")"
    if [ -n "$run_id" ]; then
      run_state="$(native_resource_run_status "$run_id")"
      run_status="${run_state%%$'\t'*}"
      run_conclusion="${run_state#*$'\t'}"
      if [ "$run_status" = "completed" ] && [ "$run_conclusion" != "success" ]; then
        info "Latest ${NATIVE_RESOURCE_WORKFLOW} run ${run_id} for ${head_sha} concluded ${run_conclusion}; dispatching a fresh run"
        stale_run_id="$run_id"
        run_id=""
      else
        info "Waiting for existing ${NATIVE_RESOURCE_WORKFLOW} run ${run_id} for ${head_sha}"
        gh run watch "$run_id" --exit-status
      fi
    fi
  fi
  if [ -z "$run_id" ]; then
    info "No successful or running ${NATIVE_RESOURCE_WORKFLOW} native-resource run found for ${head_sha}"
    info "Dispatching ${NATIVE_RESOURCE_WORKFLOW} on ${git_ref} to build cross-platform JVM native resources"
    gh workflow run "$NATIVE_RESOURCE_WORKFLOW" --ref "$git_ref" -f "version=${VERSION}"
    run_id="$(wait_for_native_resource_run_id "$head_sha" "$stale_run_id")"
    gh run watch "$run_id" --exit-status
  fi

  download_kotlin_native_resources_from_run "$run_id"
  if ! kotlin_native_resources_are_complete; then
    fail "downloaded JVM native resources are incomplete; check GitHub Actions run ${run_id}"
  fi
}

prepare_android_ndk_home() {
  if [ -n "${ANDROID_NDK_HOME:-}" ]; then
    return
  fi
  if [ -n "${ANDROID_HOME:-}" ] && [ -d "${ANDROID_HOME}/ndk/${ANDROID_NDK_VERSION}" ]; then
    export ANDROID_NDK_HOME="${ANDROID_HOME}/ndk/${ANDROID_NDK_VERSION}"
    return
  fi
  fail "ANDROID_NDK_HOME is not set and ${ANDROID_HOME:-\$ANDROID_HOME}/ndk/${ANDROID_NDK_VERSION} was not found"
}

write_checksums() {
  local root="$1"
  node - "$root" <<'NODE'
const { createHash } = require("node:crypto");
const { readdirSync, readFileSync, statSync, writeFileSync } = require("node:fs");
const { join } = require("node:path");

const root = process.argv[2];
const checksumExtensions = new Set([".md5", ".sha1", ".sha256", ".sha512"]);
const algorithms = [
  ["md5", ".md5"],
  ["sha1", ".sha1"],
  ["sha256", ".sha256"],
  ["sha512", ".sha512"],
];

function walk(directory) {
  const entries = [];
  for (const dirent of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, dirent.name);
    if (dirent.isDirectory()) {
      entries.push(...walk(path));
    } else if (dirent.isFile()) {
      entries.push(path);
    }
  }
  return entries;
}

if (!statSync(root).isDirectory()) {
  throw new Error(`${root} is not a directory`);
}

for (const path of walk(root)) {
  if (checksumExtensions.has(path.slice(path.lastIndexOf(".")))) {
    continue;
  }
  const bytes = readFileSync(path);
  for (const [algorithm, extension] of algorithms) {
    writeFileSync(`${path}${extension}`, `${createHash(algorithm).update(bytes).digest("hex")}\n`);
  }
}
NODE
}

validate_bundle_files() {
  local version="$1"
  local jvm_dir="${BUNDLE_ROOT}/me/really/codec/${version}"
  local android_dir="${BUNDLE_ROOT}/me/really/codec-android/${version}"

  require_dir "$jvm_dir"
  require_dir "$android_dir"

  for file in \
    "${jvm_dir}/codec-${version}.jar" \
    "${jvm_dir}/codec-${version}-sources.jar" \
    "${jvm_dir}/codec-${version}-javadoc.jar" \
    "${jvm_dir}/codec-${version}.pom" \
    "${jvm_dir}/codec-${version}.module" \
    "${android_dir}/codec-android-${version}.aar" \
    "${android_dir}/codec-android-${version}-sources.jar" \
    "${android_dir}/codec-android-${version}.pom" \
    "${android_dir}/codec-android-${version}.module"; do
    require_file "$file"
    require_file "${file}.asc"
    require_file "${file}.md5"
    require_file "${file}.sha1"
    require_file "${file}.sha256"
    require_file "${file}.sha512"
    require_file "${file}.asc.md5"
    require_file "${file}.asc.sha1"
    require_file "${file}.asc.sha256"
    require_file "${file}.asc.sha512"
  done

  local jvm_jar="${jvm_dir}/codec-${version}.jar"
  require_zip_entry "$jvm_jar" "me/really/codec/native/linux-x86_64/libreallyme_codec_ffi.so"
  require_zip_entry "$jvm_jar" "me/really/codec/native/linux-aarch64/libreallyme_codec_ffi.so"
  require_zip_entry "$jvm_jar" "me/really/codec/native/macos-x86_64/libreallyme_codec_ffi.dylib"
  require_zip_entry "$jvm_jar" "me/really/codec/native/macos-aarch64/libreallyme_codec_ffi.dylib"
  require_zip_entry "$jvm_jar" "me/really/codec/native/windows-x86_64/reallyme_codec_ffi.dll"
  require_zip_entry "$jvm_jar" "me/really/codec/native/native-manifest.json"

  local android_aar="${android_dir}/codec-android-${version}.aar"
  require_zip_entry "$android_aar" "jni/arm64-v8a/libreallyme_codec_ffi.so"
  require_zip_entry "$android_aar" "jni/armeabi-v7a/libreallyme_codec_ffi.so"
  require_zip_entry "$android_aar" "jni/x86_64/libreallyme_codec_ffi.so"
  require_zip_entry "$android_aar" "jni/x86/libreallyme_codec_ffi.so"
  require_zip_entry "$android_aar" "assets/reallyme-codec/native-manifest.json"
}

sign_bundle_files() {
  local root="$1"
  local file

  find "$root" -type f -name '*.asc' -delete
  while IFS= read -r -d '' file; do
    case "$file" in
      *.md5|*.sha1|*.sha256|*.sha512)
        continue
        ;;
    esac
    gpg \
      --batch \
      --yes \
      --pinentry-mode loopback \
      --passphrase-fd 0 \
      --local-user "$MAVEN_SIGNING_KEY_ID" \
      --armor \
      --detach-sign \
      --output "${file}.asc" \
      "$file" <<<"$MAVEN_SIGNING_PASSWORD"
  done < <(find "$root" -type f -print0)
}

remove_local_repository_metadata() {
  local root="$1"
  find "$root" -type f -name 'maven-metadata.xml*' -delete
}

require_tool cargo
require_tool find
require_tool grep
require_tool gpg
require_tool jar
require_tool node
require_tool rustup
require_tool sed
require_tool sleep
require_tool zip

if [ -z "${MAVEN_SIGNING_PASSWORD:-}" ]; then
  fail "MAVEN_SIGNING_PASSWORD must contain the GPG private key passphrase"
fi
if [ -z "${MAVEN_SIGNING_KEY_ID:-}" ]; then
  fail "MAVEN_SIGNING_KEY_ID must contain the GPG secret key id or fingerprint"
fi
GPG_SIGN_ARGS=(
  --batch
  --yes
  --pinentry-mode
  loopback
  --passphrase-fd
  0
  --armor
  --detach-sign
  --output
  /dev/null
  --local-user
  "$MAVEN_SIGNING_KEY_ID"
)
if ! printf '%s' "$MAVEN_SIGNING_PASSWORD" | gpg "${GPG_SIGN_ARGS[@]}" >/dev/null 2>&1; then
  fail "GPG could not sign with the configured key id and passphrase; check MAVEN_SIGNING_KEY_ID and MAVEN_SIGNING_PASSWORD"
fi

VERSION="${MAVEN_RELEASE_VERSION:-$(read_gradle_version "${ROOT_DIR}/packages/kotlin/build.gradle.kts")}"
ANDROID_VERSION="$(read_gradle_version "${ROOT_DIR}/packages/kotlin-android/build.gradle.kts")"
if [ -z "$VERSION" ]; then
  fail "unable to read JVM package version"
fi
if [ "$VERSION" != "$ANDROID_VERSION" ]; then
  fail "JVM package version $VERSION does not match Android package version $ANDROID_VERSION"
fi
if [[ "$VERSION" == *SNAPSHOT* ]]; then
  fail "Central Portal release bundles must not use SNAPSHOT versions"
fi

require_file "$GRADLE"

info "Using version ${VERSION}"
info "Using JVM native resources from ${KOTLIN_NATIVE_RESOURCES_DIR}"
ensure_kotlin_native_resources
info "Writing JVM native checksum manifest"
node "${ROOT_DIR}/scripts/write_native_manifest.mjs" \
  "$KOTLIN_NATIVE_RESOURCES_DIR" \
  "${KOTLIN_NATIVE_RESOURCES_DIR}/me/really/codec/native/native-manifest.json"

while IFS= read -r native_file; do
  require_file "$native_file"
done < <(kotlin_native_resource_files)
require_file "${KOTLIN_NATIVE_RESOURCES_DIR}/me/really/codec/native/native-manifest.json"

if [ -z "$ANDROID_JNI_LIBS_DIR_WAS_SET" ]; then
  prepare_android_ndk_home
  info "Building Android JNI libraries into ${ANDROID_JNI_LIBS_DIR}"
  rm -rf "$ANDROID_JNI_LIBS_DIR"
  "${ROOT_DIR}/scripts/build_android_native_resources.sh" "$ANDROID_JNI_LIBS_DIR"
else
  require_dir "$ANDROID_JNI_LIBS_DIR"
  info "Using Android JNI libraries from ${ANDROID_JNI_LIBS_DIR}"
fi

info "Writing Android native checksum manifest"
rm -rf "$ANDROID_NATIVE_ASSETS_DIR"
node "${ROOT_DIR}/scripts/write_native_manifest.mjs" \
  "$ANDROID_JNI_LIBS_DIR" \
  "${ANDROID_NATIVE_ASSETS_DIR}/reallyme-codec/native-manifest.json"

rm -rf \
  "${ROOT_DIR}/packages/kotlin/build/repos/releases" \
  "${ROOT_DIR}/packages/kotlin-android/build/repos/releases" \
  "$BUNDLE_ROOT" \
  "$OUTPUT_DIR"
mkdir -p "$BUNDLE_ROOT" "$OUTPUT_DIR"

info "Publishing JVM artifacts to the local release repository"
(
  unset MAVEN_SIGNING_KEY MAVEN_SIGNING_PASSWORD
"$GRADLE" -p "${ROOT_DIR}/packages/kotlin" \
    publishMavenPublicationToLocalReleaseRepository \
    -Preallyme.codec.nativeResourcesDir="$KOTLIN_NATIVE_RESOURCES_DIR" \
    -Preallyme.codec.requireFullNativeResources=true
)

info "Publishing Android artifacts to the local release repository"
(
  unset MAVEN_SIGNING_KEY MAVEN_SIGNING_PASSWORD
"$GRADLE" -p "${ROOT_DIR}/packages/kotlin-android" \
    publishReleasePublicationToLocalReleaseRepository \
    -Preallyme.codec.androidJniLibsDir="$ANDROID_JNI_LIBS_DIR" \
    -Preallyme.codec.androidNativeAssetsDir="$ANDROID_NATIVE_ASSETS_DIR" \
    -Preallyme.codec.requireAndroidJniLibs=true
)

info "Assembling Maven repository-layout bundle"
cp -R "${ROOT_DIR}/packages/kotlin/build/repos/releases/." "$BUNDLE_ROOT/"
cp -R "${ROOT_DIR}/packages/kotlin-android/build/repos/releases/." "$BUNDLE_ROOT/"

info "Removing local Maven repository metadata"
remove_local_repository_metadata "$BUNDLE_ROOT"

info "Signing Maven bundle files with local GPG"
sign_bundle_files "$BUNDLE_ROOT"

info "Writing checksums for artifacts and signatures"
write_checksums "$BUNDLE_ROOT"

info "Validating bundle contents"
validate_bundle_files "$VERSION"

BUNDLE_ZIP="${OUTPUT_DIR}/reallyme-maven-central-${VERSION}.zip"
rm -f "$BUNDLE_ZIP"
info "Creating ${BUNDLE_ZIP}"
(
  cd "$BUNDLE_ROOT"
  COPYFILE_DISABLE=1 zip -X -q -r "$BUNDLE_ZIP" .
)

require_file "$BUNDLE_ZIP"
info "Maven Central upload bundle ready:"
printf '%s\n' "$BUNDLE_ZIP"
