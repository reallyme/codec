// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/**
 * SHA-256 of `bytes` as lowercase hexadecimal, with no platform dependency.
 *
 * The generated WASM glue must verify its module bytes synchronously in
 * `initSync`, where Web Crypto is unavailable. This function is injected into
 * the glue verbatim, so it is self-contained and references nothing outside
 * its own body. The build checks it against `node:crypto` before injecting.
 */
export function reallymeCodecSha256Hex(bytes) {
  const roundConstants = new Uint32Array([
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
  ]);
  const state = new Uint32Array([
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
  ]);
  const length = bytes.length;
  if (!Number.isSafeInteger(length) || length < 0 || length > 0x7fff_ffff) {
    throw new TypeError("SHA-256 input length is outside the supported range");
  }
  const paddedLength = Math.ceil((length + 9) / 64) * 64;
  const padded = new Uint8Array(paddedLength);
  padded.set(bytes);
  padded[length] = 0x80;
  const view = new DataView(padded.buffer);
  view.setUint32(paddedLength - 8, Math.floor(length / 0x2000_0000));
  view.setUint32(paddedLength - 4, (length * 8) >>> 0);

  const schedule = new Uint32Array(64);
  const rotate = (value, bits) => (value >>> bits) | (value << (32 - bits));
  for (let block = 0; block < paddedLength; block += 64) {
    for (let index = 0; index < 16; index += 1) {
      schedule[index] = view.getUint32(block + index * 4);
    }
    for (let index = 16; index < 64; index += 1) {
      const early = schedule[index - 15];
      const late = schedule[index - 2];
      const sigma0 = rotate(early, 7) ^ rotate(early, 18) ^ (early >>> 3);
      const sigma1 = rotate(late, 17) ^ rotate(late, 19) ^ (late >>> 10);
      schedule[index] = (schedule[index - 16] + sigma0 + schedule[index - 7] + sigma1) >>> 0;
    }
    let a = state[0];
    let b = state[1];
    let c = state[2];
    let d = state[3];
    let e = state[4];
    let f = state[5];
    let g = state[6];
    let h = state[7];
    for (let index = 0; index < 64; index += 1) {
      const sum1 = rotate(e, 6) ^ rotate(e, 11) ^ rotate(e, 25);
      const choose = (e & f) ^ (~e & g);
      const first = (h + sum1 + choose + roundConstants[index] + schedule[index]) >>> 0;
      const sum0 = rotate(a, 2) ^ rotate(a, 13) ^ rotate(a, 22);
      const majority = (a & b) ^ (a & c) ^ (b & c);
      const second = (sum0 + majority) >>> 0;
      h = g;
      g = f;
      f = e;
      e = (d + first) >>> 0;
      d = c;
      c = b;
      b = a;
      a = (first + second) >>> 0;
    }
    state[0] = (state[0] + a) >>> 0;
    state[1] = (state[1] + b) >>> 0;
    state[2] = (state[2] + c) >>> 0;
    state[3] = (state[3] + d) >>> 0;
    state[4] = (state[4] + e) >>> 0;
    state[5] = (state[5] + f) >>> 0;
    state[6] = (state[6] + g) >>> 0;
    state[7] = (state[7] + h) >>> 0;
  }
  return Array.from(state, (word) => word.toString(16).padStart(8, "0")).join("");
}

/**
 * Glue source that admits only module bytes whose SHA-256 equals `digest`.
 *
 * Bytes are copied before hashing, so a shared or caller-retained buffer
 * cannot change between verification and compilation. Compiled
 * `WebAssembly.Module` objects are refused because their bytes cannot be
 * checked.
 */
export const wasmIntegritySource = (digest) => {
  if (!/^[0-9a-f]{64}$/u.test(digest)) {
    throw new TypeError("WASM digest must be 64 lowercase hexadecimal characters");
  }
  return `const __REALLYME_CODEC_WASM_SHA256 = '${digest}';
${reallymeCodecSha256Hex.toString()}
function __reallymeVerifiedWasmBytes(source) {
    let view;
    if (source instanceof ArrayBuffer) {
        view = new Uint8Array(source);
    } else if (ArrayBuffer.isView(source)) {
        view = new Uint8Array(source.buffer, source.byteOffset, source.byteLength);
    } else {
        throw new TypeError('ReallyMe codec WASM must be supplied as bytes');
    }
    const bytes = view.slice();
    if (reallymeCodecSha256Hex(bytes) !== __REALLYME_CODEC_WASM_SHA256) {
        throw new TypeError('ReallyMe codec WASM bytes do not match this package');
    }
    return bytes;
}
`;
};
