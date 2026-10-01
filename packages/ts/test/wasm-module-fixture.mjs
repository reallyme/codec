// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

export * from "../dist/wasm/reallyme_codec_wasm.js?fixture-original";
import { ReallyMeCodecError } from "../dist/errors.js";

let operationHandler;
let jsonOperationHandler;
let voidHandler = () => undefined;
let cidHandler = () => undefined;
let initialized = true;

export const setInitialized = (value) => {
  initialized = value;
};

export const dagCborCodecCode = () => {
  if (!initialized) {
    throw new TypeError("WASM module is not initialized");
  }
  return 0x71;
};

export const setOperationHandler = (handler) => {
  operationHandler = handler;
  jsonOperationHandler = handler;
};

export const setJsonOperationHandler = (handler) => {
  jsonOperationHandler = handler;
};

export const setVoidHandler = (handler) => {
  voidHandler = handler;
};

export const requireSupportedMulticodec = (name) => voidHandler(name);

export const setCidHandler = (handler) => {
  cidHandler = handler;
};

export const tryParseCid = (cid) => cidHandler(cid);

export const processOperation = (request) => {
  if (operationHandler === undefined) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return operationHandler(request);
};

export const processOperationJson = (request) => {
  if (jsonOperationHandler === undefined) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return jsonOperationHandler(request);
};
