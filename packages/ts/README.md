# @reallyme/codec

TypeScript codecs backed by Rust WebAssembly. Node.js requires version 20.19
or newer; browser use requires WebAssembly support.

```sh
npm install @reallyme/codec
```

## Usage

Initialize the bundled Wasm module before installing the provider:

```ts
import { readFileSync } from "node:fs";
import { ReallyMeCodec, installReallyMeCodecWasmProvider } from "@reallyme/codec";
import * as wasm from "@reallyme/codec/wasm/reallyme_codec_wasm.js";

const wasmBytes = readFileSync(
  new URL(import.meta.resolve("@reallyme/codec/wasm/reallyme_codec_wasm_bg.wasm")),
);
wasm.initSync({ module: wasmBytes });
installReallyMeCodecWasmProvider(wasm);

const encoded = ReallyMeCodec.base64urlEncode(new Uint8Array([1, 2, 3]));
const decoded = ReallyMeCodec.base64urlDecode(encoded);
```

Browser initialization accepts Wasm file bytes, a `Response` or a URL. Use the
Wasm file from the same package version; precompiled `WebAssembly.Module`
objects are rejected because their bytes cannot be verified.

Pass ordinary `Uint8Array` values. Node `Buffer`, typed-array subclasses and
proxies are rejected; copy them into a plain `Uint8Array` first.

CBOR builders use `bigint` for integers and entry lists for maps:

```ts
import { ReallyMeDeterministicCbor } from "@reallyme/codec";

const value = ReallyMeDeterministicCbor.mapText([
  ["b", ReallyMeDeterministicCbor.unsigned(2n)],
  ["a", ReallyMeDeterministicCbor.bytes(new Uint8Array([0, 1, 2]))],
]);
const cbor = ReallyMeCodec.deterministicCborEncode(value);
```

See [format limits](../../README.md#formats-and-limits) before selecting JCS,
CBOR or CID operations. Clear sensitive returned arrays with `fill(0)`;
JavaScript strings and garbage-collected objects cannot be reliably erased.

`ReallyMeCodecError.code` classifies failures. Its generated `reason` and
`origin` identify validated protobuf errors; both are undefined when no valid
error envelope was received.

`processOperation` accepts binary protobuf and `processOperationJson` accepts
ProtoJSON request bytes. Both return binary protobuf responses. Generated
messages are exported through `@reallyme/codec/proto`; see the
[protobuf API](../../docs/protobuf.md).

## Test

From the repository root:

```sh
npm ci --prefix packages/ts
npm --prefix packages/ts test
npm --prefix packages/ts run test:browser:built
```

## License

MIT or Apache-2.0, at your option. See [LICENSE](LICENSE).
