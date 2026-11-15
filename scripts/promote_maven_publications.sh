#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail
umask 077

fail() {
    printf 'Maven promotion failed: %s\n' "$1" >&2
    exit 1
}

if [[ $# != 3 || "$1" != /* || ! "$2" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ || ! "$3" =~ ^[0-9a-f]{40}$ ]]; then
    fail 'expected an absolute package directory, stable version, and full source SHA'
fi

readonly packages="$1" version="$2" source_sha="$3"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
work_dir="$(mktemp -d)"
readonly script_dir work_dir
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

: "${MAVEN_SIGNING_KEY:?Maven signing key required}"
: "${MAVEN_SIGNING_PASSWORD:?Maven signing passphrase required}"
printf '%s\n' "$MAVEN_SIGNING_KEY" | gpg --batch --import >/dev/null 2>&1 ||
    fail 'could not import the Maven signing key'
unset MAVEN_SIGNING_KEY
# The isolated keyring must contain exactly one signing identity.
fingerprints="$(gpg --batch --with-colons --list-secret-keys |
    awk -F: '$1 == "sec" { primary = 1; next } primary && $1 == "fpr" { print $10; primary = 0 }')"
[[ "$fingerprints" =~ ^[A-F0-9]{40}$ ]] || fail 'the signing keyring must contain one primary key'

shopt -s nullglob
for file in "$bundle"/me/really/*/"$version"/*; do
    gpg --batch --pinentry-mode loopback --passphrase-fd 0 --local-user "$fingerprints" \
        --armor --detach-sign --output "$file.asc" "$file" <<<"$MAVEN_SIGNING_PASSWORD"
    for signed_file in "$file" "$file.asc"; do
        for algorithm in md5 sha1 sha256 sha512; do
            "${algorithm}sum" -- "$signed_file" | cut -d ' ' -f 1 >"$signed_file.$algorithm"
        done
    done
done
unset MAVEN_SIGNING_PASSWORD

readonly archive="$work_dir/reallyme-maven-central-$version.zip"
(cd "$bundle" && zip -X -q -r "$archive" me)
node "$script_dir/upload_maven_central_bundle.mjs" "$archive" "$version"
