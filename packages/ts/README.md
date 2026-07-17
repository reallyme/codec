# @reallyme/codec

The `@reallyme/codec` TypeScript package provides codecs through WebAssembly.

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
| DAG-CBOR and CID | `dagCborEncode`, `dagCborDecode`, `dagCborComputeCid`, `dagCborVerifyCid`, content hash and multihash helpers |
| JCS | `canonicalizeJson`, `canonicalizeJsonText` |
| PEM | wipeable `Uint8Array` armor through `encodePem`, `decodePem`, with strict label and size policy |
| Protobuf | `processProto`, `processProtoJson`, and `@reallyme/codec/proto` generated types |

`processProto` accepts binary generated `CodecOperationRequest` bytes.
`processProtoJson` accepts UTF-8 bytes containing the generated ProtoJSON view
of that same message. Both return binary `CodecProtoResultEnvelope` bytes;
expected validation failures are represented in the envelope and are not
thrown as backend exceptions.

Errors are typed as `ReallyMeCodecError`; they do not include raw input bytes
or backend exception text.

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
