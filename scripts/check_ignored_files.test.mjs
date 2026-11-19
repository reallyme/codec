// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("./check_ignored_files.mjs", import.meta.url));
const repositoryRoot = fileURLToPath(new URL("../", import.meta.url));
// Fixture commits retain the repository's existing identity rather than
// inventing an author or depending on CI's global Git configuration.
const identityResult = spawnSync("git", ["-C", repositoryRoot, "show", "-s", "--format=%an%x00%ae", "HEAD"], {
  encoding: "utf8",
});
assert.equal(identityResult.status, 0, identityResult.stderr);
const [authorName, authorEmail] = identityResult.stdout.trimEnd().split("\0");
assert.ok(authorName && authorEmail);

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "codec-ignore-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const env = {
    ...process.env,
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_CONFIG_GLOBAL: join(root, "no-global-config"),
    GIT_AUTHOR_NAME: authorName,
    GIT_AUTHOR_EMAIL: authorEmail,
    GIT_COMMITTER_NAME: authorName,
    GIT_COMMITTER_EMAIL: authorEmail,
  };
  // A hook can supply its own index and worktree; fixtures must not inherit it.
  for (const name of ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_PREFIX"]) {
    delete env[name];
  }
  const git = (...args) => {
    const result = spawnSync("git", args, { cwd: root, env, encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout;
  };
  git("init", "--quiet", "--initial-branch=main");
  const baselineRules = "AGENTS.md\n.claude/\n.codex/\n.local/\n.build/\n";
  const write = (path, contents) => {
    if (path === ".gitignore") contents = baselineRules + contents;
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), contents);
  };
  write(".gitignore", "");
  git("add", ".gitignore");
  const checkInput = (input, ...args) => spawnSync(process.execPath, [script, ...args], {
    cwd: root, env, encoding: "utf8", input,
  });
  const check = (...args) => checkInput(undefined, ...args);
  return { root, env, git, write, check, checkInput };
}

test("clean index passes and explicit negations remain supported", (t) => {
  const { git, write, check } = fixture(t);
  write(".gitignore", "*.env\n!example.env\n");
  write("example.env", "public example\n");
  write("private.env", "local only\n");
  git("add", ".gitignore", "example.env");
  assert.equal(check().status, 0);
});

test("staged work reports fail even when unstaged bytes look like a README", (t) => {
  const { git, write, check } = fixture(t);
  write("README.md", "# Task summary\n\nPrivate working notes.\n");
  git("add", "README.md");
  write("README.md", "# Public package\n");
  const result = check();
  assert.equal(result.status, 1);
  assert.match(result.stderr, /TRANSIENT_DOCUMENTATION_CONTENT/u);
});

test("custom public guides require a staged declaration", (t) => {
  const { git, write, check } = fixture(t);
  write("docs/api.md", "# Public API\n");
  git("add", "docs/api.md");
  write(".github/documentation-policy.json", JSON.stringify({
    schema: 1,
    documents: [{
      path: "docs/api.md", kind: "reference",
      purpose: "Describe the supported public API and compatibility guarantees.",
    }],
  }));
  assert.equal(check().status, 1);
  git("add", ".github/documentation-policy.json");
  assert.equal(check().status, 0);
});

test("force-added ignored files fail including newline paths", (t) => {
  const { git, write, check } = fixture(t);
  write(".gitignore", "*.env\n");
  write("private\nvalue.env", "local only\n");
  git("add", ".gitignore");
  git("add", "--force", "private\nvalue.env");
  const result = check();
  assert.equal(result.status, 1);
  assert.match(result.stderr, /"private\\nvalue\.env"/u);
});

test("new ignore rules reject files that were already tracked", (t) => {
  const { git, write, check } = fixture(t);
  write("private.env", "local only\n");
  git("add", "private.env");
  write(".gitignore", "*.env\n");
  git("add", ".gitignore");
  assert.equal(check().status, 1);
});

test("unstaged rule removal cannot hide a staged ignored file", (t) => {
  const { git, write, check } = fixture(t);
  write(".gitignore", "*.env\n");
  write("private.env", "local only\n");
  git("add", ".gitignore");
  git("add", "--force", "private.env");
  write(".gitignore", "");
  assert.equal(check().status, 1);
});

test("nested staged ignore rules and negations are evaluated", (t) => {
  const { git, write, check } = fixture(t);
  write(".gitignore", "*.env\n");
  write("nested/.gitignore", "!public.env\n");
  write("nested/public.env", "public example\n");
  git("add", ".gitignore", "nested");
  assert.equal(check().status, 0);
  write("nested/private.env", "local only\n");
  git("add", "--force", "nested/private.env");
  assert.equal(check().status, 1);
});

test("untracked and local exclude rules do not change shared policy", (t) => {
  const { root, git, write, check } = fixture(t);
  write("allowed.txt", "public\n");
  git("add", "allowed.txt");
  write(".gitignore", "*.txt\n");
  writeFileSync(join(root, ".git/info/exclude"), "*.txt\n");
  assert.equal(check().status, 0);
});

test("an alternate staged index is respected", (t) => {
  const { root, env, git, write, check } = fixture(t);
  write(".gitignore", "*.env\n");
  write("private.env", "local only\n");
  git("add", ".gitignore");
  const alternate = join(root, "alternate.index");
  copyFileSync(join(root, ".git/index"), alternate);
  const alternateEnv = { ...env, GIT_INDEX_FILE: alternate };
  const result = spawnSync("git", ["add", "--force", "private.env"], {
    cwd: root, env: alternateEnv, encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(check().status, 0);
  const alternateResult = spawnSync(process.execPath, [script], {
    cwd: root, env: alternateEnv, encoding: "utf8",
  });
  assert.equal(alternateResult.status, 1);
});

test("repository rules reject local instructions and tool/build directories", (t) => {
  const { git, write, check } = fixture(t);
  const rules = readFileSync(new URL("../.gitignore", import.meta.url), "utf8");
  write(".gitignore", rules);
  git("add", ".gitignore");
  const directories = rules.split("\n").filter((rule) => /^\.[a-z][a-z0-9-]*\/$/u.test(rule));
  const paths = ["AGENTS.md", ...directories.flatMap((directory) => [
    `${directory}file`, `nested/${directory}file`,
  ])];
  for (const path of paths) {
    write(path, "local only\n");
    git("add", "--force", path);
    const result = check();
    assert.equal(result.status, 1, path);
    assert.ok(result.stderr.includes(JSON.stringify(path)), result.stderr);
    git("rm", "--cached", path);
  }
});

test("symlink ignore files are rejected", { skip: process.platform === "win32" }, (t) => {
  const { root, git, write, check } = fixture(t);
  write("rules", "*.env\n");
  rmSync(join(root, ".gitignore"));
  symlinkSync("rules", join(root, ".gitignore"));
  git("add", ".gitignore", "rules");
  const result = check();
  assert.equal(result.status, 1);
  assert.match(result.stderr, /IGNORE_FILE_MUST_BE_REGULAR/u);
});

test("unsupported arguments and missing repositories fail closed", (t) => {
  const { root, check } = fixture(t);
  assert.equal(check("--unknown").status, 1);
  rmSync(join(root, ".git"), { recursive: true, force: true });
  assert.equal(check().status, 1);
});

test("history rejects ignored files even when a later commit removes them", (t) => {
  const { git, write, check } = fixture(t);
  write(".gitignore", "*.env\n");
  write("private\nvalue.env", "local only\n");
  git("add", ".gitignore");
  git("add", "--force", "private\nvalue.env");
  git("-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Add fixture files");
  git("rm", "private\nvalue.env");
  git("-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Remove fixture file");
  assert.equal(check().status, 0);
  const result = check("--history");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /"private\\nvalue\.env"/u);
});

test("history rejects work reports deleted before the pushed tip", (t) => {
  const { git, write, check } = fixture(t);
  write("README.md", "# Task summary\n\nPrivate working notes.\n");
  git("add", "README.md");
  git("-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Add fixture files");
  git("rm", "README.md");
  git("-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Update fixture files");
  assert.equal(check().status, 0);
  const result = check("--history");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /TRANSIENT_DOCUMENTATION_CONTENT/u);
});

test("clean complete history passes while shallow history fails closed", (t) => {
  const { root, git, write, check } = fixture(t);
  write(".gitignore", "*.env\n");
  write("public.txt", "public\n");
  git("add", ".gitignore", "public.txt");
  git("-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Add fixture files");
  assert.equal(check("--history").status, 0);
  writeFileSync(join(root, ".git/shallow"), `${git("rev-parse", "HEAD").trim()}\n`);
  const result = check("--history");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /COMPLETE_HISTORY_REQUIRED/u);
});

test("push validation checks the pushed branch rather than the checked-out branch", (t) => {
  const { git, write, check, checkInput } = fixture(t);
  write(".gitignore", "*.env\n");
  git("add", ".gitignore");
  git("-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Add fixture rules");
  git("checkout", "--quiet", "-b", "private-work");
  write("private.env", "local only\n");
  git("add", "--force", "private.env");
  git("-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Add fixture file");
  const tip = git("rev-parse", "HEAD").trim();
  git("checkout", "--quiet", "main");
  assert.equal(check("--history").status, 0);
  const result = checkInput(`refs/heads/private-work ${tip} refs/heads/private-work ${"0".repeat(40)}\n`, "--pre-push");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /private\.env/u);
});

test("push validation handles tags, deletions and malformed inputs", (t) => {
  const { git, write, checkInput } = fixture(t);
  write(".gitignore", "*.env\n");
  git("add", ".gitignore");
  git("-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "Add fixture rules");
  git("-c", "user.name=" + authorName, "-c", "user.email=" + authorEmail, "-c", "tag.gpgsign=false", "tag", "--annotate", "fixture", "-m", "Fixture tag");
  const tag = git("rev-parse", "refs/tags/fixture").trim();
  assert.equal(checkInput(`refs/tags/fixture ${tag} refs/tags/fixture ${"0".repeat(40)}\n`, "--pre-push").status, 0);
  assert.equal(checkInput(`(delete) ${"0".repeat(40)} refs/tags/fixture ${tag}\n`, "--pre-push").status, 0);
  const invalid = checkInput("refs/heads/main --all refs/heads/main invalid\n", "--pre-push");
  assert.equal(invalid.status, 1);
  assert.match(invalid.stderr, /INVALID_PUSH_UPDATES/u);
});
