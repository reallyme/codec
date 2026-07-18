// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const packageDirectory = resolve(scriptDirectory, "..");
const browserResultPrefix = "__REALLYME_CODEC_BROWSER_WASM_RESULT__";
const browserTestTimeoutMs = 45_000;
const chromeStartupTimeoutMs = 15_000;

const chromeCandidates = [
  process.env.REALLYME_CODEC_CHROME_PATH,
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  "/Applications/Chromium.app/Contents/MacOS/Chromium",
  "/usr/bin/google-chrome",
  "/usr/bin/google-chrome-stable",
  "/usr/bin/chromium",
  "/usr/bin/chromium-browser",
].filter((candidate) => typeof candidate === "string" && candidate.length > 0);

const fail = (message) => {
  process.stderr.write(`${message}\n`);
  process.exit(1);
};

const chromeExecutable = chromeCandidates.find((candidate) => existsSync(candidate));
if (chromeExecutable === undefined) {
  fail("Chrome or Chromium is required for the browser WASM release gate.");
}

if (typeof WebSocket !== "function") {
  fail("Node.js with a global WebSocket implementation is required.");
}

const mimeType = (path) => {
  switch (extname(path)) {
    case ".html":
      return "text/html; charset=utf-8";
    case ".js":
    case ".mjs":
      return "text/javascript; charset=utf-8";
    case ".wasm":
      return "application/wasm";
    case ".json":
      return "application/json; charset=utf-8";
    default:
      return "application/octet-stream";
  }
};

const browserTestPage = () => `<!doctype html>
<meta charset="utf-8">
<script type="importmap">
{
  "imports": {
    "@bufbuild/protobuf": "/node_modules/@bufbuild/protobuf/dist/esm/index.js",
    "@bufbuild/protobuf/codegenv2": "/node_modules/@bufbuild/protobuf/dist/esm/codegenv2/index.js"
  }
}
</script>
<script type="module">
const resultPrefix = ${JSON.stringify(browserResultPrefix)};
const report = (result) => {
  console.log(resultPrefix + JSON.stringify(result));
};
const assert = (condition, code) => {
  if (!condition) {
    throw new Error(code);
  }
};

try {
  const wasm = await import("/dist/wasm/reallyme_codec_wasm.js");
  const codec = await import("/dist/index.js");
  await wasm.default();
  codec.installReallyMeCodecWasmProvider(wasm);

  const bytes = new Uint8Array([1, 2, 3, 4]);
  assert(codec.base64Encode(bytes) === "AQIDBA==", "base64-browser-result");
  assert(codec.base64urlEncode(bytes) === "AQIDBA", "base64url-browser-result");
  assert(codec.bytesToLowerHex(bytes) === "01020304", "hex-browser-result");

  const encoded = codec.deterministicCborEncode({
    type: "map",
    value: [
      {
        key: { type: "text", value: "answer" },
        value: {
          type: "integer",
          value: { type: "unsigned", value: 42n },
        },
      },
    ],
  });
  assert(encoded instanceof Uint8Array, "deterministic-encode-type");
  assert(Array.from(encoded).join(",") === "161,102,97,110,115,119,101,114,24,42", "deterministic-encode-bytes");
  const decoded = codec.deterministicCborDecode(encoded);
  assert(decoded.type === "map", "deterministic-decode-map");
  assert(decoded.value.length === 1, "deterministic-decode-map-length");
  assert(decoded.value[0].key.type === "text", "deterministic-decode-key-type");
  assert(decoded.value[0].key.value === "answer", "deterministic-decode-key-value");
  assert(decoded.value[0].value.type === "integer", "deterministic-decode-value-type");
  assert(decoded.value[0].value.value.type === "unsigned", "deterministic-decode-integer-type");
  assert(decoded.value[0].value.value.value === 42n, "deterministic-decode-integer-value");

  let rejected = false;
  try {
    codec.deterministicCborDecode(new Uint8Array([0x18, 0x01]));
  } catch (error) {
    rejected = error instanceof codec.ReallyMeCodecError && error.code === "invalid-input";
  }
  assert(rejected, "deterministic-invalid-browser-rejection");

  report({ ok: true });
} catch (error) {
  report({
    ok: false,
    message: error instanceof Error ? error.message : "browser wasm test failed",
  });
}
</script>`;

const isPathInside = (root, target) => {
  const relative = target.slice(root.length);
  return target === root || (target.startsWith(root) && relative.startsWith(sep));
};

const startStaticServer = () =>
  new Promise((resolveServer, rejectServer) => {
    const server = createServer((request, response) => {
      const url = new URL(request.url ?? "/", "http://127.0.0.1");
      if (url.pathname === "/browser-wasm-test.html") {
        response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        response.end(browserTestPage());
        return;
      }

      const relativePath = decodeURIComponent(url.pathname).replace(/^\/+/u, "");
      const normalizedPath = resolve(packageDirectory, relativePath);
      if (!isPathInside(packageDirectory, normalizedPath) || basename(normalizedPath) === "") {
        response.writeHead(404);
        response.end();
        return;
      }
      try {
        const body = readFileSync(normalizedPath);
        response.writeHead(200, { "Content-Type": mimeType(normalizedPath) });
        response.end(body);
      } catch {
        response.writeHead(404);
        response.end();
      }
    });
    server.once("error", rejectServer);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (address === null || typeof address === "string") {
        rejectServer(new Error("browser test server did not bind a TCP port"));
        return;
      }
      resolveServer({ server, port: address.port });
    });
  });

const fetchJson = async (url) => {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error("Chrome DevTools endpoint was not ready");
  }
  return response.json();
};

const waitForPageDebuggerUrl = async (port) => {
  const deadline = Date.now() + chromeStartupTimeoutMs;
  while (Date.now() < deadline) {
    try {
      const targets = await fetchJson(`http://127.0.0.1:${port}/json/list`);
      if (Array.isArray(targets)) {
        const page = targets.find(
          (target) =>
            target?.type === "page" &&
            typeof target.webSocketDebuggerUrl === "string",
        );
        if (page !== undefined) {
          return page.webSocketDebuggerUrl;
        }
      }
    } catch {
      await new Promise((resolveTimer) => setTimeout(resolveTimer, 100));
    }
  }
  throw new Error("Chrome page DevTools endpoint did not become ready");
};

class ChromeSession {
  constructor(socket) {
    this.nextId = 1;
    this.pending = new Map();
    this.handlers = new Map();
    this.socket = socket;
    this.socket.addEventListener("message", (event) => this.handleMessage(event));
  }

  command(method, params = {}) {
    const id = this.nextId;
    this.nextId += 1;
    const message = JSON.stringify({ id, method, params });
    return new Promise((resolveCommand, rejectCommand) => {
      this.pending.set(id, { resolveCommand, rejectCommand });
      this.socket.send(message);
    });
  }

  on(method, handler) {
    this.handlers.set(method, handler);
  }

  handleMessage(event) {
    const message = JSON.parse(event.data);
    if (typeof message.id === "number") {
      const pending = this.pending.get(message.id);
      if (pending !== undefined) {
        this.pending.delete(message.id);
        if (message.error !== undefined) {
          pending.rejectCommand(new Error(message.error.message));
        } else {
          pending.resolveCommand(message.result);
        }
      }
      return;
    }
    const handler = this.handlers.get(message.method);
    if (handler !== undefined) {
      handler(message.params);
    }
  }
}

const connectChrome = (url) =>
  new Promise((resolveSocket, rejectSocket) => {
    const socket = new WebSocket(url);
    socket.addEventListener("open", () => resolveSocket(socket), { once: true });
    socket.addEventListener(
      "error",
      () => rejectSocket(new Error("Chrome DevTools WebSocket failed")),
      { once: true },
    );
  });

const waitForChromeExit = (chrome) =>
  new Promise((resolveClose) => {
    if (chrome.exitCode !== null || chrome.signalCode !== null) {
      resolveClose();
      return;
    }
    const timeout = setTimeout(resolveClose, 2_000);
    chrome.once("close", () => {
      clearTimeout(timeout);
      resolveClose();
    });
  });

const runBrowserTest = async ({ serverPort, debuggerPort }) => {
  const debuggerUrl = await waitForPageDebuggerUrl(debuggerPort);
  const socket = await connectChrome(debuggerUrl);
  const session = new ChromeSession(socket);
  const failures = [];
  let browserResult;

  session.on("Runtime.consoleAPICalled", (event) => {
    const text = event.args
      .map((argument) => argument.value)
      .filter((value) => typeof value === "string")
      .join(" ");
    if (text.startsWith(browserResultPrefix)) {
      browserResult = JSON.parse(text.slice(browserResultPrefix.length));
    }
    if (event.type === "error") {
      failures.push(text);
    }
  });
  session.on("Runtime.exceptionThrown", (event) => {
    failures.push(event.exceptionDetails?.text ?? "browser exception");
  });

  await session.command("Runtime.enable");
  await session.command("Page.enable");
  await session.command("Page.navigate", {
    url: `http://127.0.0.1:${serverPort}/browser-wasm-test.html`,
  });

  const deadline = Date.now() + browserTestTimeoutMs;
  while (browserResult === undefined && Date.now() < deadline) {
    await new Promise((resolveTimer) => setTimeout(resolveTimer, 100));
  }
  socket.close();

  if (browserResult === undefined) {
    throw new Error("browser WASM test timed out");
  }
  if (browserResult.ok !== true) {
    throw new Error(browserResult.message ?? "browser WASM test failed");
  }
  if (failures.length > 0) {
    throw new Error("browser console reported an error");
  }
};

const run = async () => {
  const { server, port: serverPort } = await startStaticServer();
  const userDataDir = mkdtempSync(resolve(tmpdir(), "reallyme-codec-chrome-"));
  const chrome = spawn(chromeExecutable, [
    "--headless=new",
    "--disable-gpu",
    "--disable-dev-shm-usage",
    "--no-first-run",
    "--no-default-browser-check",
    "--no-sandbox",
    "--remote-debugging-port=0",
    `--user-data-dir=${userDataDir}`,
    "about:blank",
  ], {
    stdio: ["ignore", "ignore", "pipe"],
  });

  let debuggerPort;
  chrome.stderr.setEncoding("utf8");
  chrome.stderr.on("data", (chunk) => {
    const match = /DevTools listening on ws:\/\/127\.0\.0\.1:(\d+)\//u.exec(chunk);
    if (match !== null) {
      debuggerPort = Number(match[1]);
    }
  });

  try {
    const deadline = Date.now() + chromeStartupTimeoutMs;
    while (debuggerPort === undefined && Date.now() < deadline) {
      await new Promise((resolveTimer) => setTimeout(resolveTimer, 100));
    }
    if (debuggerPort === undefined) {
      throw new Error("Chrome did not report a DevTools port");
    }
    await runBrowserTest({ serverPort, debuggerPort });
  } finally {
    chrome.kill();
    await waitForChromeExit(chrome);
    server.close();
    rmSync(userDataDir, {
      force: true,
      maxRetries: 5,
      recursive: true,
      retryDelay: 100,
    });
  }
};

try {
  await run();
  process.stdout.write("browser WASM release gate passed\n");
} catch (error) {
  fail(error instanceof Error ? error.message : "browser WASM release gate failed");
}
