#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail
set +x
umask 077

fail() {
    printf 'Maven promotion failed: %s\n' "$1" >&2
    exit 1
}

if [[ $# != 3 || "$1" != /* || ! "$2" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ || ! "$3" =~ ^[0-9a-f]{40}$ ]]; then
    fail 'expected an absolute package directory, stable version, and full source SHA'
fi

# Keep credentials local until the individual signing or upload operation.
export -n signing_key signing_password portal_username portal_password
: "${MAVEN_SIGNING_KEY:?Maven signing key required}"
: "${MAVEN_SIGNING_PASSWORD:?Maven signing passphrase required}"
: "${CENTRAL_PORTAL_USERNAME:?Maven Central username required}"
: "${CENTRAL_PORTAL_PASSWORD:?Maven Central password required}"
signing_key="$MAVEN_SIGNING_KEY"
signing_password="$MAVEN_SIGNING_PASSWORD"
readonly portal_username="$CENTRAL_PORTAL_USERNAME" portal_password="$CENTRAL_PORTAL_PASSWORD"
unset MAVEN_SIGNING_KEY MAVEN_SIGNING_PASSWORD CENTRAL_PORTAL_USERNAME CENTRAL_PORTAL_PASSWORD

readonly packages="$1" version="$2" source_sha="$3"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
work_dir="$(mktemp -d)"
readonly script_dir work_dir
cd -- "$script_dir/.."
export GNUPGHOME="$work_dir/gnupg"

cleanup() {
    gpgconf --kill gpg-agent >/dev/null 2>&1 || true
    rm -rf -- "$work_dir"
}
trap cleanup EXIT

mkdir -m 700 "$GNUPGHOME"
readonly bundle="$work_dir/bundle"
mkdir -p "$bundle"

node "$script_dir/stage_maven_publications.mjs" "$packages" "$version" "$source_sha" "$bundle"
node "$script_dir/run_pinned_release_readiness.mjs" --check-pins
node "$script_dir/../.release-readiness/scripts/repository-privacy.mjs" --artifact "$bundle"

gpg --batch --import >/dev/null 2>&1 <<<"$signing_key" ||
    fail 'could not import the Maven signing key'
unset signing_key
# The isolated keyring must contain exactly one signing identity.
fingerprints="$(gpg --batch --with-colons --list-secret-keys |
    awk -F: '$1 == "sec" { primary = 1; next } primary && $1 == "fpr" { print $10; primary = 0 }')"
[[ "$fingerprints" =~ ^[A-F0-9]{40}$ ]] || fail 'the signing keyring must contain one primary key'

shopt -s nullglob
for file in "$bundle"/me/really/*/"$version"/*; do
    gpg --batch --pinentry-mode loopback --passphrase-fd 0 --local-user "$fingerprints" \
        --armor --detach-sign --output "$file.asc" "$file" <<<"$signing_password"
    gpg --batch --no-tty --no-auto-key-retrieve --verify "$file.asc" "$file" >/dev/null 2>&1
    for signed_file in "$file" "$file.asc"; do
        for algorithm in md5 sha1 sha256 sha512; do
            "${algorithm}sum" -- "$signed_file" | cut -d ' ' -f 1 >"$signed_file.$algorithm"
        done
    done
done
unset signing_password

readonly archive="$work_dir/reallyme-maven-central-$version.zip"
(cd "$bundle" && zip -X -q -r "$archive" me)
node "$script_dir/../.release-readiness/scripts/repository-privacy.mjs" --artifact "$archive"
CENTRAL_PORTAL_USERNAME="$portal_username" CENTRAL_PORTAL_PASSWORD="$portal_password" \
    node "$script_dir/upload_maven_central_bundle.mjs" "$archive" "$version"
