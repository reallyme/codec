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

It deliberately does not interpret DER, ASN.1, or key structure. Use this crate
when you need the text envelope only. Algorithm-aware key import belongs in the
crypto layer that consumes the decoded bytes.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [LICENSE](LICENSE) for both license texts.
