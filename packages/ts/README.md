# @reallyme/codec

The `@reallyme/codec` TypeScript package provides base encodings, multiformats, CBOR, CID, PEM and JCS through WebAssembly.

`ReallyMeCodecError` retains its presentation `code` and exposes the generated
`reason` and `origin` from validated protobuf error envelopes. Import
`CodecErrorReason` and `CodecErrorOrigin` from this package to inspect them.
Failures without a validated envelope leave both fields undefined.

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

Initialize the bundled Wasm module before installing its provider. Unexpected runtime failures return `provider-failure`; recovery is attempted on a later call. Precompiled `WebAssembly.Module` inputs are rejected; supply bytes, a `Response` or a URL from the matching package.

## Surface

| Family | APIs |
|---|---|
| Base encodings | `base64Encode`, `base64Decode`, `base64urlEncode`, `base64urlDecode`, `base64urlDecodeBytes`, `bytesToLowerHex`, `lowerHexToBytes` |
| Multiformats | `base58btcEncode`, `base58btcDecode`, `multibase*`, `multicodec*`, `multikey*`, binding validation |
| Deterministic CBOR | typed `deterministicCborEncode` and `deterministicCborDecode` for the bounded RFC 8949 §4.2.3 length-first profile |
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
separate profile restricted to text map keys and signed 64-bit integers.

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
mutable-byte boundary for browser, Node, and WASM callers. Pass ordinary
`Uint8Array` views; subclasses (including Node `Buffer`), proxies, and overridden
byte metadata are rejected. The facade copies bytes through captured prototype
methods and does not invoke a caller-owned copy method. Create a plain `Uint8Array` copy
when adapting other byte containers.

PEM decoding accepts up to 1.5 MiB of armor, including the base64 expansion and
line wrapping of a 1 MiB DER body. Smaller `maxInputLen` and `maxDerLen` policies
remain enforced by Rust. Private-key labels require an explicit allowlist.

JCS is a strict RFC 8785 subset. It rejects duplicate properties, unpaired
UTF-16 surrogates, Unicode noncharacters, and integer-valued numbers outside
the exact JavaScript integer range; this includes the RFC example `1e19`.
CID validation accepts
CIDv0 and supported multibase CIDv1 strings up to 1024 UTF-8 bytes; paths and
trailing decoded bytes are rejected. CID computation uses DAG-CBOR/SHA-256
CIDv1 in lowercase base32 and hashes the supplied bytes without parsing CBOR.
CID verification validates one canonical DAG-CBOR block before comparing the
CID and requires the supplied CID text to use that canonical lowercase base32
form. `dagCborVerifyCid` returns a boolean; `dagCborVerifyCidDetails` returns
the canonical CID diagnostics. Invalid DAG-CBOR returns a typed canonicalization
error. Tagged values and floating-point encodings outside this package's closed
DAG-CBOR model return `invalid-input`.

Returned arrays belong to the caller. Clear sensitive arrays with `fill(0)` after use. Strings, garbage-collected objects and runtime-created copies cannot be reliably erased; keep sensitive values short-lived and out of logs.

Errors are typed as `ReallyMeCodecError`; they do not include raw input bytes
or backend exception text.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [LICENSE](LICENSE) for both license texts.
