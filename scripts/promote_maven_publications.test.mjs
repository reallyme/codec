// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("promote_maven_publications.sh", import.meta.url));

for (const failure of ["privacy", "signature"]) {
  test(`Maven promotion stops before upload on ${failure} failure and isolates credentials`, (t) => {
    const directory = mkdtempSync(join(tmpdir(), "codec-promotion-test-"));
    t.after(() => rmSync(directory, { recursive: true, force: true }));
    const scripts = join(directory, "scripts");
    const bin = join(directory, "bin");
    const events = join(directory, "events");
    mkdirSync(scripts);
    mkdirSync(bin);
    copyFileSync(script, join(scripts, "promote.sh"));
    writeFileSync(join(bin, "node"), `#!/bin/bash
set -euo pipefail
[[ -z "\${CENTRAL_PORTAL_USERNAME+x}" && -z "\${CENTRAL_PORTAL_PASSWORD+x}" && -z "\${MAVEN_SIGNING_KEY+x}" && -z "\${MAVEN_SIGNING_PASSWORD+x}" ]] || exit 91
case "$1" in
  */stage_maven_publications.mjs)
    mkdir -p "$5/me/really/codec/0.4.6"
    printf 'fixture' > "$5/me/really/codec/0.4.6/codec-0.4.6.pom"
    printf 'stage\\n' >> "$TEST_EVENTS" ;;
  */run_pinned_release_readiness.mjs) printf 'authenticate\\n' >> "$TEST_EVENTS" ;;
  */repository-privacy.mjs)
    printf 'privacy\\n' >> "$TEST_EVENTS"
    [[ "$TEST_FAILURE" != privacy ]] ;;
  */upload_maven_central_bundle.mjs) printf 'unexpected-upload\\n' >> "$TEST_EVENTS"; exit 92 ;;
  *) exit 93 ;;
esac
`, { mode: 0o700 });
    writeFileSync(join(bin, "gpg"), `#!/bin/bash
set -euo pipefail
[[ -z "\${CENTRAL_PORTAL_USERNAME+x}" && -z "\${CENTRAL_PORTAL_PASSWORD+x}" && -z "\${MAVEN_SIGNING_KEY+x}" && -z "\${MAVEN_SIGNING_PASSWORD+x}" ]] || exit 94
case " $* " in
  *" --import "*)
    read -r key
    [[ "$key" == private-signing-key ]]
    printf 'import\\n' >> "$TEST_EVENTS" ;;
  *" --list-secret-keys "*)
    printf 'sec:::::::::\\nfpr:::::::::0123456789ABCDEF0123456789ABCDEF01234567:\\n' ;;
  *" --detach-sign "*)
    read -r password
    [[ "$password" == private-signing-password ]]
    printf 'sign\\n' >> "$TEST_EVENTS" ;;
  *" --verify "*)
    [[ " $* " == *" --no-auto-key-retrieve "* ]]
    printf 'verify\\n' >> "$TEST_EVENTS"
    exit 1 ;;
  *) exit 95 ;;
esac
`, { mode: 0o700 });
    writeFileSync(join(bin, "gpgconf"), "#!/bin/bash\nexit 0\n", { mode: 0o700 });
    const result = spawnSync("bash", ["-x", join(scripts, "promote.sh"), directory, "0.4.6", "a".repeat(40)], {
      encoding: "utf8", timeout: 10_000,
      env: {
        ...process.env, PATH: `${bin}:${process.env.PATH}`,
        TEST_EVENTS: events, TEST_FAILURE: failure,
        MAVEN_SIGNING_KEY: "private-signing-key", MAVEN_SIGNING_PASSWORD: "private-signing-password",
        CENTRAL_PORTAL_USERNAME: "private-portal-user", CENTRAL_PORTAL_PASSWORD: "private-portal-password",
      },
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, 1, result.stderr);
    const expected = ["stage", "authenticate", "privacy"];
    if (failure === "signature") expected.push("import", "sign", "verify");
    assert.deepEqual(readFileSync(events, "utf8").trim().split("\n"), expected);
    assert.doesNotMatch(`${result.stdout}${result.stderr}`, /private-(?:signing|portal)/u);
  });
}
