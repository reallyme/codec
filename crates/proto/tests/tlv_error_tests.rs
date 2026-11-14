// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! TLV error mapping and sensitive generated ownership guarantees.
#![cfg(feature = "generated")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "enumerated public error contract"
)]

use buffa::{Enumeration, Message};
use reallyme_codec_proto::generated::proto::reallyme::codec::v1 as proto;
use reallyme_codec_proto::{codec_api_error, decode_json, CodecWireError};

#[test]
fn tlv_reason_mapping_is_total_and_preserves_detail_and_origin() {
    let reasons = proto::CodecTlvErrorReason::values();
    assert_eq!(reasons.len(), 17);
    for &detail in reasons {
        let value = detail.to_i32();
        let error = codec_api_error(CodecWireError::from_tlv_reason(detail));
        if value == 0 {
            assert_eq!(
                error.origin.as_known(),
                Some(proto::CodecErrorOrigin::CODEC_ERROR_ORIGIN_PROVIDER)
            );
            assert!(matches!(
                error.error.as_ref(),
                Some(proto::codec_api_error::Error::Backend(_))
            ));
            continue;
        }
        assert_eq!(
            error.origin.as_known(),
            Some(
                if detail == proto::CodecTlvErrorReason::ArithmeticOverflow {
                    proto::CodecErrorOrigin::CODEC_ERROR_ORIGIN_PROVIDER
                } else {
                    proto::CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER
                }
            )
        );
        let Some(proto::codec_api_error::Error::Tlv(error)) = error.error.as_ref() else {
            panic!("missing TLV error detail")
        };
        assert_eq!(error.detail.as_known(), Some(detail));
        let expected = match detail {
            proto::CodecTlvErrorReason::InvalidProfile
            | proto::CodecTlvErrorReason::InvalidLimits => {
                proto::CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF
            }
            proto::CodecTlvErrorReason::InputLimit
            | proto::CodecTlvErrorReason::IdentifierLimit
            | proto::CodecTlvErrorReason::LengthLimit => {
                proto::CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED
            }
            proto::CodecTlvErrorReason::ArithmeticOverflow => {
                proto::CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL
            }
            _ => proto::CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_INVALID_TLV,
        };
        assert_eq!(error.reason.as_known(), Some(expected));
    }
}

#[test]
fn generated_tlv_owners_redact_and_clear_sensitive_fields() {
    fn owner<T: zeroize::ZeroizeOnDrop>() {}
    owner::<proto::CodecTlvSourceRequest>();
    owner::<proto::CodecTlvHeader>();
    owner::<proto::CodecTlvFrame>();
    owner::<proto::CodecTlvLength>();
    let mut request: proto::CodecTlvSourceRequest = decode_json(br#"{"encoded":"AQID"}"#).unwrap();
    assert!(format!("{request:?}").contains("[REDACTED]"));
    request.clear();
    assert!(request.encoded.is_empty());
    let mut header: proto::CodecTlvHeader =
        decode_json(br#"{"declaredValueLength":"18446744073709551615"}"#).unwrap();
    assert_eq!(header.declared_value_length, u64::MAX);
    assert!(format!("{header:?}").contains("[REDACTED]"));
    header.clear();
    assert_eq!(header.declared_value_length, 0);
    let mut length: proto::CodecTlvLength =
        decode_json(br#"{"declaredValueLength":"18446744073709551615"}"#).unwrap();
    assert_eq!(length.declared_value_length, u64::MAX);
    assert!(format!("{length:?}").contains("[REDACTED]"));
    length.clear();
    assert_eq!(length.declared_value_length, 0);
}
