#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe LLC
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

fail() {
    printf 'workflow lint installation failed: %s\n' "$1" >&2
    exit 1
}

[[ "$#" -eq 0 ]] || fail 'arguments are not supported'
[[ -n "${RUNNER_TEMP:-}" && -d "${RUNNER_TEMP}" ]] || fail 'runner temporary directory is unavailable'
[[ -n "${GITHUB_PATH:-}" && -f "${GITHUB_PATH}" && ! -L "${GITHUB_PATH}" ]] || fail 'runner path file is unavailable'

readonly version='1.7.12'
if ! host_os="$(uname -s)" || ! host_arch="$(uname -m)"; then
    fail 'host platform could not be identified'
fi

# Review the upstream release checksums with the version. Fetching checksums
# alongside an archive would not authenticate the artifact against this policy.
case "${host_os}/${host_arch}" in
    Linux/x86_64)
        platform='linux_amd64'
        digest='8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8'
        ;;
    Linux/aarch64 | Linux/arm64)
        platform='linux_arm64'
        digest='325e971b6ba9bfa504672e29be93c24981eeb1c07576d730e9f7c8805afff0c6'
        ;;
    Darwin/x86_64)
        platform='darwin_amd64'
        digest='5b44c3bc2255115c9b69e30efc0fecdf498fdb63c5d58e17084fd5f16324c644'
        ;;
    Darwin/arm64)
        platform='darwin_arm64'
        digest='aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f'
        ;;
    *) fail 'host platform is unsupported' ;;
esac

umask 077
if ! work_dir="$(mktemp -d "${RUNNER_TEMP%/}/workflow-lint.XXXXXX")"; then
    fail 'private download directory could not be created'
fi
trap 'rm -rf -- "${work_dir}"' EXIT

archive="${work_dir}/actionlint_${version}_${platform}.tar.gz"
url="https://github.com/rhysd/actionlint/releases/download/v${version}/actionlint_${version}_${platform}.tar.gz"
if ! curl --fail --silent --show-error --location --proto '=https' \
    --connect-timeout 10 --max-time 120 --retry 3 --retry-max-time 240 \
    --output "${archive}" "${url}"; then
    fail 'pinned release could not be downloaded'
fi
if ! printf '%s  %s\n' "${digest}" "${archive}" | shasum -a 256 --check --status; then
    fail 'pinned release checksum does not match'
fi

if ! tar -xzf "${archive}" -C "${work_dir}" actionlint; then
    fail 'verified release could not be extracted'
fi
if [[ ! -f "${work_dir}/actionlint" || -L "${work_dir}/actionlint" ]]; then
    fail 'verified executable is not a regular file'
fi
if ! installed_version="$("${work_dir}/actionlint" -version)"; then
    fail 'verified executable could not be run'
fi
[[ "${installed_version%%$'\n'*}" == "${version}" ]] || fail 'executable version does not match'

install_dir="${RUNNER_TEMP%/}/workflow-lint-bin"
[[ ! -L "${install_dir}" ]] || fail 'installation directory must not be a symlink'
if ! mkdir -p "${install_dir}" || ! install -m 0755 "${work_dir}/actionlint" "${install_dir}/actionlint"; then
    fail 'verified executable could not be installed'
fi
if ! printf '%s\n' "${install_dir}" >> "${GITHUB_PATH}"; then
    fail 'runner path could not be updated'
fi
printf 'actionlint %s installed from verified release bytes\n' "${version}"
