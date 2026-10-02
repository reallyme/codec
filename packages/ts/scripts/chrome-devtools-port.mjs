// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync } from "node:fs";

const MAX_TCP_PORT = 65_535;

export const readDevToolsActivePort = (path) => {
  let contents;
  try {
    contents = readFileSync(path, "utf8");
  } catch (error) {
    if (error !== null && typeof error === "object" && error.code === "ENOENT") {
      return undefined;
    }
    throw error;
  }

  // Chrome writes the selected port on the first line. Read that file rather
  // than matching individual stderr chunks, which may split the announcement.
  const firstLine = contents.split(/\r?\n/u, 1)[0];
  if (!/^\d{1,5}$/u.test(firstLine)) {
    throw new Error("Chrome wrote an invalid DevTools port");
  }
  const port = Number(firstLine);
  if (port < 1 || port > MAX_TCP_PORT) {
    throw new Error("Chrome wrote an invalid DevTools port");
  }
  return port;
};
