<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# @reallyme/codec

`@reallyme/codec` is the TypeScript package for the Rust `reallyme-codec`
surface. It exposes the same codec families through a WASM-backed facade:
base64, unpadded base64url, lowercase hex, multibase, multicodec, multikey,
deterministic generic CBOR, DAG-CBOR/CID helpers, JCS, PEM armor, and the
`reallyme.codec.v1` protobuf contract.

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

`processOperation` accepts binary generated `CodecOperationRequest` bytes.
`processOperationJson` accepts UTF-8 bytes containing the generated ProtoJSON
view of that same message. Both return binary `CodecOperationResponse` bytes;
expected validation failures are represented by its typed error outcome and
are not thrown as backend exceptions. Operation-specific public methods use
that same fully discriminated response internally; the package does not keep
operation-specific `*Proto` helper APIs or hand-written structured JSON result
paths.

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

Encoded CBOR and decoded byte-string values can contain the complete sensitive
document. Returned buffers belong to the caller and should be cleared with
`fill(0)` as soon as they are no longer needed. The package snapshots mutable
inputs and clears its mutable request, response, and intermediate buffers on
success and failure. JavaScript strings, garbage-collected object graphs, and
runtime-internal protobuf storage cannot be deterministically erased; callers
should therefore keep sensitive values short-lived and out of logs and
telemetry.

Swift and Kotlin/Java expose the same generic `processOperation` and
`processOperationJson` methods through the Rust C/JNI boundary. The method
names and generated response semantics are intentionally identical across
SDKs.

Errors are typed as `ReallyMeCodecError`; they do not include raw input bytes
or backend exception text.
