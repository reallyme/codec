# reallyme-codec-pem

`reallyme-codec-pem` parses and emits PEM text armor: BEGIN/END labels,
base64 bodies, line ending normalization, strict label matching, and size
limits.

This is a bounded PEM armor profile rather than a strict RFC 7468 generator
or parser. Encoding defaults to 64-character body lines but accepts an explicit
width from 1 through 76. Decoding joins base64 body lines of any width and
permits empty lines; boundary lines must match exactly, without trailing
whitespace. Callers that require strict RFC 7468 input should enforce that
profile before passing armor to this crate.

PEM decoding does not validate DER, ASN.1 values or cryptographic keys.

`decode_pem_bundle` accepts certificates, CRLs and other labeled armor as
ordered, schema-generated guarded blocks. Options use
`CodecPemBundleDecodeOptions`; zero bounds select 1.5 MiB of input, 1 MiB
of decoded data per block, and 64 blocks. Labels and decoded bytes clear
on drop, including when a later block fails.

The default strict profile requires nonempty uppercase labels, single label
separators, newline-delimited boundaries, and base64 body lines. An explicit
compatibility profile retains inline armor, surrounding prose, unknown labels,
Unicode body whitespace, legacy colon-delimited headers and incomplete trailing
armor. `FIRST` stops after the first complete block; `ALL` validates every
complete block. Certificate decoding and chain policy remain the caller's
responsibility. The same implementation backs the generated
`CodecApiOperationRequest.pem_bundle_decode` operation.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [LICENSE](LICENSE) for both license texts.
