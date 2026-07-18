# @reallyme/codec

The `@reallyme/codec` TypeScript package provides base encodings, multiformats, CBOR, CID, PEM and JCS through WebAssembly.

```sh
npm install @reallyme/codec
```

## Usage

Load the WASM module once at process startup, then use the typed facade.

```ts
import { readFileSync } from "node:fs";
import {
  ReallyMeCodec,
  installReallyMeCodecWasmProvider,
} from "@reallyme/codec";
import * as wasm from "@reallyme/codec/wasm/reallyme_codec_wasm.js";

const wasmBytes = readFileSync(
  new URL(import.meta.resolve("@reallyme/codec/wasm/reallyme_codec_wasm_bg.wasm")),
);

wasm.initSync({ module: wasmBytes });
installReallyMeCodecWasmProvider(wasm);

const encoded = ReallyMeCodec.base64urlEncode(new Uint8Array([1, 2, 3]));
const decoded = ReallyMeCodec.base64urlDecode(encoded);
```

## Surface

| Family | APIs |
|---|---|
| Base encodings | `base64Encode`, `base64Decode`, `base64urlEncode`, `base64urlDecode`, `base64urlDecodeBytes`, `bytesToLowerHex`, `lowerHexToBytes` |
| Multiformats | `base58btcEncode`, `base58btcDecode`, `multibase*`, `multicodec*`, `multikey*`, binding validation |
| Deterministic CBOR | typed `deterministicCborEncode` and `deterministicCborDecode` for the bounded RFC 8949 profile |
| DAG-CBOR and CID | `dagCborEncode`, `dagCborDecode`, `dagCborComputeCid`, `dagCborVerifyCid`, content hash and multihash helpers |
| JCS | `canonicalizeJson`, `canonicalizeJsonText` |
| PEM | wipeable `Uint8Array` armor through `encodePem`, `decodePem`, with strict label and size policy |
| Protobuf | `processOperation`, `processOperationJson`, and `@reallyme/codec/proto` generated types |

`processOperation` accepts binary `CodecOperationRequest` bytes; `processOperationJson` accepts ProtoJSON request bytes. Both return binary `CodecOperationResponse` bytes. Validation failures are carried by its typed error outcome.

The deterministic-CBOR API uses a closed recursive value model with exact
`bigint` integers, `Uint8Array` byte strings, arrays, and entry-list maps with
integer or text keys. It does not use JavaScript `number` for integers. Floats,
tags, indefinite-length items, arbitrary simple values, and
compound map keys are outside the supported profile. DAG-CBOR remains a
separate, stricter profile and retains its existing APIs and behavior.

```ts
const value = ReallyMeDeterministicCbor.mapText([
  ["b", ReallyMeDeterministicCbor.unsigned(2n)],
  ["a", ReallyMeDeterministicCbor.bytes(new Uint8Array([0, 1, 2]))],
]);
const encoded = ReallyMeCodec.deterministicCborEncode(value);

const dag = ReallyMeDagCbor.mapText([
  ["payload", ReallyMeDagCbor.bytes(new Uint8Array([0, 1, 2]))],
]);
const dagBytes = ReallyMeCodec.dagCborEncode(dag);
```

Encoding canonicalizes map ordering. Decoding rejects duplicate semantic keys,
non-canonical input, unsupported CBOR types, and values beyond the documented
resource limits. DAG-CBOR builders expose text-key maps and byte/integer
helpers; deterministic CBOR additionally supports integer-key maps and the
complete documented `u64`/`i64` integer ranges. `Uint8Array` is the canonical
mutable-byte boundary for browser, Node, and WASM callers.

Returned arrays belong to the caller. Clear sensitive arrays with `fill(0)` after use. Strings, garbage-collected objects and runtime-created copies cannot be reliably erased; keep sensitive values short-lived and out of logs.

Errors are typed as `ReallyMeCodecError`; they do not include raw input bytes
or backend exception text.

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
