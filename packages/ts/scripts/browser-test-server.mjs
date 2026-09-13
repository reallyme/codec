// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { createServer } from "node:http";
import { readFileSync, realpathSync } from "node:fs";
import { extname, resolve, sep } from "node:path";

const isInside = (root, path) => path.startsWith(`${root}${sep}`);

export const startStaticServer = ({ packageDirectory, testPage }) =>
  new Promise((resolveServer, rejectServer) => {
    // Only the built package and its protobuf runtime are needed by the page.
    // Serving the checkout root would expose local .npmrc/.env files and symlinks.
    const assetRoots = [
      resolve(packageDirectory, "dist"),
      resolve(packageDirectory, "node_modules/@bufbuild/protobuf/dist/esm"),
    ];
    const server = createServer((request, response) => {
      const rejectRequest = (status) => {
        response.writeHead(status);
        response.end();
      };
      // Binding loopback alone does not prevent a rebinding hostname from
      // reaching it. The test page always uses this exact local authority.
      if (request.headers.host !== `127.0.0.1:${request.socket.localPort}`) {
        rejectRequest(403);
        return;
      }
      if (request.method !== "GET") {
        rejectRequest(405);
        return;
      }
      let pathname;
      try {
        pathname = decodeURIComponent(new URL(request.url ?? "/", "http://127.0.0.1").pathname);
      } catch {
        rejectRequest(400);
        return;
      }
      if (pathname === "/browser-wasm-test.html") {
        response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        response.end(testPage());
        return;
      }
      const path = resolve(packageDirectory, pathname.replace(/^\/+/u, ""));
      const root = assetRoots.find((candidate) => isInside(candidate, path));
      const extension = extname(path);
      if (root === undefined || ![".js", ".mjs", ".wasm"].includes(extension)) {
        rejectRequest(404);
        return;
      }
      try {
        const realPath = realpathSync(path);
        if (!isInside(realpathSync(root), realPath)) {
          rejectRequest(404);
          return;
        }
        const body = readFileSync(realPath);
        response.writeHead(200, {
          "Content-Type": extension === ".wasm" ? "application/wasm" : "text/javascript; charset=utf-8",
          "X-Content-Type-Options": "nosniff",
        });
        response.end(body);
      } catch {
        rejectRequest(404);
      }
    });
    server.once("error", rejectServer);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (address === null || typeof address === "string") {
        server.close();
        rejectServer(new Error("browser test server did not bind a TCP port"));
        return;
      }
      resolveServer({ server, port: address.port });
    });
  });
