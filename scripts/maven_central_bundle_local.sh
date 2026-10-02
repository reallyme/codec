#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe LLC
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail
IFS=$'\n\t'

# Usage:
#   MAVEN_SIGNING_KEY_ID=<long-gpg-key-id-or-fingerprint> \
#   MAVEN_SIGNING_PASSWORD="..." \
#   KOTLIN_NATIVE_RESOURCES_DIR=/path/to/full/kotlin-native-resources \
#   ANDROID_NDK_HOME=/path/to/android-ndk \
#   ./scripts/maven_central_bundle_local.sh
#
# MAVEN_NATIVE_RESOURCE_RUN_ID=<github-actions-run-id> selects a successful
# Kotlin/Android preflight for the current main commit. When omitted, the
# latest run for the current commit and package version is selected.
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
JVM_LOCAL_RELEASE_REPOSITORY_DIR="${JVM_LOCAL_RELEASE_REPOSITORY_DIR:-${ROOT_DIR}/packages/kotlin/build/repos/releases}"
ANDROID_LOCAL_RELEASE_REPOSITORY_DIR="${ANDROID_LOCAL_RELEASE_REPOSITORY_DIR:-${ROOT_DIR}/packages/kotlin-android/build/repos/releases}"
NATIVE_RESOURCE_WORKFLOW="${MAVEN_NATIVE_RESOURCE_WORKFLOW:-kotlin-android-package-preflight.yml}"
NATIVE_RESOURCE_ARTIFACT_PATTERN="${MAVEN_NATIVE_RESOURCE_ARTIFACT_PATTERN:-kotlin-native-*}"
NATIVE_RESOURCE_DOWNLOAD_DIR="${MAVEN_NATIVE_RESOURCE_DOWNLOAD_DIR:-${WORK_DIR}/kotlin-native-artifacts}"
NATIVE_RESOURCE_RUN_ID="${MAVEN_NATIVE_RESOURCE_RUN_ID:-}"

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

find_latest_workflow_run() {
  local workflow="$1"
  local title="$2"
  local head_sha="$3"
  gh run list \
    --repo reallyme/codec \
    --workflow "$workflow" \
    --commit "$head_sha" \
    --limit 100 \
    --json databaseId,displayTitle,event,headSha \
    --jq ".[] | select(.headSha == \"${head_sha}\" and .event == \"workflow_dispatch\" and .displayTitle == \"${title}\") | .databaseId" \
    | head -n 1
}

validate_successful_workflow_run() {
  local run_id="$1"
  local workflow="$2"
  local title="$3"
  local expected_workflow_id
  local run_metadata
  local run_status
  local run_conclusion
  local run_head_sha
  local run_head_branch
  local run_workflow_id
  local run_title
  local run_event

  if [[ ! "$run_id" =~ ^[1-9][0-9]*$ ]]; then
    fail "a positive GitHub Actions run id is required for ${workflow}"
  fi
  expected_workflow_id="$(gh api "repos/reallyme/codec/actions/workflows/${workflow}" --jq '.id')"
  run_metadata="$(gh run view "$run_id" --repo reallyme/codec \
    --json status,conclusion,headSha,headBranch,workflowDatabaseId,displayTitle,event \
    --jq '[.status, .conclusion, .headSha, .headBranch, (.workflowDatabaseId | tostring), .displayTitle, .event] | @tsv')"
  IFS=$'\t' read -r run_status run_conclusion run_head_sha run_head_branch \
    run_workflow_id run_title run_event <<< "$run_metadata"
  if [ "$run_status" != "completed" ] || [ "$run_conclusion" != "success" ] || \
     [ "$run_head_sha" != "$RELEASE_SHA" ] || [ "$run_head_branch" != "main" ] || \
     [ "$run_workflow_id" != "$expected_workflow_id" ] || [ "$run_title" != "$title" ] || \
     [ "$run_event" != "workflow_dispatch" ]; then
    fail "GitHub Actions run ${run_id} does not certify ${workflow} for the current main commit and version"
  fi
}

download_kotlin_native_resources_from_run() {
  local run_id="$1"
  local artifact_dir

  rm -rf "$NATIVE_RESOURCE_DOWNLOAD_DIR"
  mkdir -p "$NATIVE_RESOURCE_DOWNLOAD_DIR" "$KOTLIN_NATIVE_RESOURCES_DIR"
  info "Downloading JVM native resource artifacts from GitHub Actions run ${run_id}"
  gh run download "$run_id" --repo reallyme/codec \
    --pattern "$NATIVE_RESOURCE_ARTIFACT_PATTERN" \
    --dir "$NATIVE_RESOURCE_DOWNLOAD_DIR"

  while IFS= read -r artifact_dir; do
    cp -R "${artifact_dir}/." "$KOTLIN_NATIVE_RESOURCES_DIR/"
  done < <(find "$NATIVE_RESOURCE_DOWNLOAD_DIR" -mindepth 1 -maxdepth 1 -type d | sort)
}

ensure_kotlin_native_resources() {
  local run_id="${NATIVE_RESOURCE_RUN_ID}"
  local latest_run_id
  local runtime_gate_run_id

  require_tool gh
  latest_run_id="$(find_latest_workflow_run "$NATIVE_RESOURCE_WORKFLOW" \
    "Kotlin Android package preflight ${VERSION}" "$RELEASE_SHA")"
  if [ -z "$run_id" ]; then
    run_id="$latest_run_id"
  fi
  if [ "$run_id" != "$latest_run_id" ]; then
    fail "selected native-resource run is not the latest preflight for the current main commit"
  fi
  validate_successful_workflow_run "$run_id" "$NATIVE_RESOURCE_WORKFLOW" \
    "Kotlin Android package preflight ${VERSION}"
  runtime_gate_run_id="$(find_latest_workflow_run "android-runtime-gate.yml" \
    "Android Runtime Gate" "$RELEASE_SHA")"
  validate_successful_workflow_run "$runtime_gate_run_id" "android-runtime-gate.yml" \
    "Android Runtime Gate"

  case "$KOTLIN_NATIVE_RESOURCES_DIR" in
    "${ROOT_DIR}"/build/*) ;;
    *) fail "native resource staging must be within the repository build directory" ;;
  esac
  if [ -L "$KOTLIN_NATIVE_RESOURCES_DIR" ]; then
    fail "native resource staging must not be a symlink"
  fi
  rm -rf "$KOTLIN_NATIVE_RESOURCES_DIR"
  download_kotlin_native_resources_from_run "$run_id"
  if ! kotlin_native_resources_are_complete; then
    fail "downloaded JVM native resources are incomplete"
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
require_tool git
require_tool gh
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

if [ -n "$(git -C "$ROOT_DIR" status --porcelain --untracked-files=all)" ]; then
  fail "release checkout must have a clean working tree"
fi
if [ "$(git -C "$ROOT_DIR" branch --show-current)" != "main" ] &&
   [ "${MAVEN_CENTRAL_CI:-}" != "1" ]; then
  fail "local release checkout must be on main"
fi
git -C "$ROOT_DIR" fetch --no-tags origin main
RELEASE_SHA="$(git -C "$ROOT_DIR" rev-parse HEAD)"
if [ "$RELEASE_SHA" != "$(git -C "$ROOT_DIR" rev-parse origin/main)" ]; then
  fail "release checkout is not the current origin/main tip"
fi
require_file "$GRADLE"

info "Using version ${VERSION}"
info "Using JVM native resources from ${KOTLIN_NATIVE_RESOURCES_DIR}"
ensure_kotlin_native_resources
if [ "$(uname -s)" = "Linux" ]; then
  "${ROOT_DIR}/scripts/verify_linux_glibc_floor.sh" "$KOTLIN_NATIVE_RESOURCES_DIR"
fi
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
  "$JVM_LOCAL_RELEASE_REPOSITORY_DIR" \
  "$ANDROID_LOCAL_RELEASE_REPOSITORY_DIR" \
  "$BUNDLE_ROOT" \
  "$OUTPUT_DIR"
mkdir -p "$BUNDLE_ROOT" "$OUTPUT_DIR"

info "Publishing JVM artifacts to the local release repository"
(
  unset MAVEN_SIGNING_KEY MAVEN_SIGNING_PASSWORD
"$GRADLE" -p "${ROOT_DIR}/packages/kotlin" \
    publishMavenPublicationToLocalReleaseRepository \
    -Preallyme.maven.localReleaseRepositoryDir="$JVM_LOCAL_RELEASE_REPOSITORY_DIR" \
    -Preallyme.codec.nativeResourcesDir="$KOTLIN_NATIVE_RESOURCES_DIR" \
    -Preallyme.codec.requireFullNativeResources=true
)

info "Publishing Android artifacts to the local release repository"
(
  unset MAVEN_SIGNING_KEY MAVEN_SIGNING_PASSWORD
"$GRADLE" -p "${ROOT_DIR}/packages/kotlin-android" \
    publishReleasePublicationToLocalReleaseRepository \
    -Preallyme.maven.localReleaseRepositoryDir="$ANDROID_LOCAL_RELEASE_REPOSITORY_DIR" \
    -Preallyme.codec.androidJniLibsDir="$ANDROID_JNI_LIBS_DIR" \
    -Preallyme.codec.androidNativeAssetsDir="$ANDROID_NATIVE_ASSETS_DIR" \
    -Preallyme.codec.requireAndroidJniLibs=true
)

info "Assembling Maven repository-layout bundle"
cp -R "${JVM_LOCAL_RELEASE_REPOSITORY_DIR}/." "$BUNDLE_ROOT/"
cp -R "${ANDROID_LOCAL_RELEASE_REPOSITORY_DIR}/." "$BUNDLE_ROOT/"

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
