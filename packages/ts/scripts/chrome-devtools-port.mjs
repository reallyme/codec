// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync } from "node:fs";

const MAX_TCP_PORT = 65_535;

const validPort = (value) => {
  if (!/^\d{1,5}$/u.test(value)) {
    return undefined;
  }
  const port = Number(value);
  return port >= 1 && port <= MAX_TCP_PORT ? port : undefined;
};

export const readDevToolsListeningPort = (stderr) => {
  const match = /DevTools listening on ws:\/\/127\.0\.0\.1:(\d{1,5})\//u.exec(stderr);
  return match === null ? undefined : validPort(match[1]);
};

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
  const port = validPort(firstLine);
  if (port === undefined) {
    throw new Error("Chrome wrote an invalid DevTools port");
  }
  return port;
};
