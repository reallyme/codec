// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated from the fixed error text in codec.proto. Do not edit.
use super::proto::reallyme::codec::v1::CodecDerErrorReason;
use super::proto::reallyme::codec::v1::CodecGeneralCborArgumentWidth;
use super::proto::reallyme::codec::v1::CodecGeneralCborErrorReason;
use super::proto::reallyme::codec::v1::CodecGeneralCborFloatPolicy;
use super::proto::reallyme::codec::v1::CodecGeneralCborFloatWidth;
use super::proto::reallyme::codec::v1::CodecGeneralCborLengthPolicy;
use super::proto::reallyme::codec::v1::CodecGeneralCborMajorType;
use super::proto::reallyme::codec::v1::CodecGeneralCborMapOrdering;
use super::proto::reallyme::codec::v1::CodecGeneralCborSerialization;
use super::proto::reallyme::codec::v1::CodecTlvErrorReason;

impl ::core::fmt::Display for CodecGeneralCborErrorReason {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_UNSPECIFIED => "unspecified error reason",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_EMPTY_INPUT => "empty input",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_TRUNCATED => "truncated item",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_RESERVED_ADDITIONAL_INFORMATION => {
                "reserved additional information"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_INDEFINITE_ARGUMENT => {
                "invalid indefinite argument"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_UNEXPECTED_BREAK => "unexpected break",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_SIMPLE_VALUE => {
                "invalid simple value encoding"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_CHUNK => "invalid string chunk",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INDEFINITE_NOT_ALLOWED => {
                "indefinite length is not allowed"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_NON_PREFERRED_ARGUMENT => {
                "non-preferred argument encoding"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_NON_PREFERRED_FLOAT => {
                "non-preferred floating-point encoding"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_FLOAT_NOT_ALLOWED => {
                "floating-point values are not allowed"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_NON_FINITE_FLOAT => {
                "non-finite floating-point value"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_UTF8 => "invalid UTF-8",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_DUPLICATE_MAP_KEY => "duplicate map key",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_MAP_KEY_ORDER => "map key ordering violation",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_TRAILING_DATA => "trailing data",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_WRONG_TYPE => "unexpected item type",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_WRONG_TAG => "unexpected tag",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_EMPTY_EMBEDDED_ITEM => "empty embedded CBOR",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_NON_CONTIGUOUS_STRING => {
                "non-contiguous string contents"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_LIMIT => "invalid resource limit",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_UNSUPPORTED_POLICY => {
                "unsupported profile combination"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INPUT_LIMIT => "input limit exceeded",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_OUTPUT_LIMIT => "output limit exceeded",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_DEPTH_LIMIT => "nesting limit exceeded",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_NODE_LIMIT => "node limit exceeded",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_ENTRY_LIMIT => "container entry limit exceeded",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_BYTE_STRING_LIMIT => {
                "byte-string content limit exceeded"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_TEXT_LIMIT => "text content limit exceeded",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_WORK_LIMIT => "validation work limit exceeded",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_CONTAINER_EXCEEDS_INPUT => {
                "container declaration exceeds remaining input"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_ARITHMETIC_OVERFLOW => {
                "length arithmetic overflow"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_ALLOCATION_FAILURE => "bounded allocation failed",
        };
        formatter.write_str(label)
    }
}

impl ::core::fmt::Debug for CodecGeneralCborErrorReason {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_UNSPECIFIED => "Unspecified",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_EMPTY_INPUT => "EmptyInput",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_TRUNCATED => "Truncated",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_RESERVED_ADDITIONAL_INFORMATION => {
                "ReservedAdditionalInformation"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_INDEFINITE_ARGUMENT => {
                "InvalidIndefiniteArgument"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_UNEXPECTED_BREAK => "UnexpectedBreak",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_SIMPLE_VALUE => "InvalidSimpleValue",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_CHUNK => "InvalidChunk",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INDEFINITE_NOT_ALLOWED => "IndefiniteNotAllowed",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_NON_PREFERRED_ARGUMENT => "NonPreferredArgument",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_NON_PREFERRED_FLOAT => "NonPreferredFloat",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_FLOAT_NOT_ALLOWED => "FloatNotAllowed",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_NON_FINITE_FLOAT => "NonFiniteFloat",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_UTF8 => "InvalidUtf8",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_DUPLICATE_MAP_KEY => "DuplicateMapKey",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_MAP_KEY_ORDER => "MapKeyOrder",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_TRAILING_DATA => "TrailingData",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_WRONG_TYPE => "WrongType",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_WRONG_TAG => "WrongTag",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_EMPTY_EMBEDDED_ITEM => "EmptyEmbeddedItem",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_NON_CONTIGUOUS_STRING => "NonContiguousString",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_LIMIT => "InvalidLimit",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_UNSUPPORTED_POLICY => "UnsupportedPolicy",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_INPUT_LIMIT => "InputLimit",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_OUTPUT_LIMIT => "OutputLimit",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_DEPTH_LIMIT => "DepthLimit",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_NODE_LIMIT => "NodeLimit",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_ENTRY_LIMIT => "EntryLimit",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_BYTE_STRING_LIMIT => "ByteStringLimit",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_TEXT_LIMIT => "TextLimit",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_WORK_LIMIT => "WorkLimit",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_CONTAINER_EXCEEDS_INPUT => {
                "ContainerExceedsInput"
            }
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_ARITHMETIC_OVERFLOW => "ArithmeticOverflow",
            Self::CODEC_GENERAL_CBOR_ERROR_REASON_ALLOCATION_FAILURE => "AllocationFailure",
        };
        formatter.write_str(label)
    }
}

impl ::std::error::Error for CodecGeneralCborErrorReason {}

impl ::core::fmt::Display for CodecDerErrorReason {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_DER_ERROR_REASON_UNSPECIFIED => "unspecified error reason",
            Self::CODEC_DER_ERROR_REASON_EMPTY_INPUT => "empty document",
            Self::CODEC_DER_ERROR_REASON_INVALID_LIMITS => "invalid limits",
            Self::CODEC_DER_ERROR_REASON_INPUT_TOO_LARGE => "input limit exceeded",
            Self::CODEC_DER_ERROR_REASON_NESTING_TOO_DEEP => "nesting limit exceeded",
            Self::CODEC_DER_ERROR_REASON_TOO_MANY_ELEMENTS => "element limit exceeded",
            Self::CODEC_DER_ERROR_REASON_INTEGER_TOO_LARGE => "integer limit exceeded",
            Self::CODEC_DER_ERROR_REASON_TRUNCATED_INPUT => "truncated input",
            Self::CODEC_DER_ERROR_REASON_INDEFINITE_LENGTH => "indefinite length",
            Self::CODEC_DER_ERROR_REASON_NON_CANONICAL_ENCODING => "noncanonical encoding",
            Self::CODEC_DER_ERROR_REASON_INVALID_INTEGER => "invalid integer",
            Self::CODEC_DER_ERROR_REASON_INVALID_TAG => "invalid tag",
            Self::CODEC_DER_ERROR_REASON_UNEXPECTED_TAG => "unexpected tag",
            Self::CODEC_DER_ERROR_REASON_INVALID_LENGTH => "invalid length",
            Self::CODEC_DER_ERROR_REASON_INVALID_VALUE => "invalid value",
            Self::CODEC_DER_ERROR_REASON_TRAILING_DATA => "trailing data",
            Self::CODEC_DER_ERROR_REASON_SET_ORDERING => "noncanonical set ordering",
            Self::CODEC_DER_ERROR_REASON_ARITHMETIC_OVERFLOW => "length arithmetic overflow",
            Self::CODEC_DER_ERROR_REASON_ALLOCATION_FAILURE => "bounded allocation failed",
        };
        formatter.write_str(label)
    }
}

impl ::core::fmt::Debug for CodecDerErrorReason {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_DER_ERROR_REASON_UNSPECIFIED => "Unspecified",
            Self::CODEC_DER_ERROR_REASON_EMPTY_INPUT => "EmptyInput",
            Self::CODEC_DER_ERROR_REASON_INVALID_LIMITS => "InvalidLimits",
            Self::CODEC_DER_ERROR_REASON_INPUT_TOO_LARGE => "InputTooLarge",
            Self::CODEC_DER_ERROR_REASON_NESTING_TOO_DEEP => "NestingTooDeep",
            Self::CODEC_DER_ERROR_REASON_TOO_MANY_ELEMENTS => "TooManyElements",
            Self::CODEC_DER_ERROR_REASON_INTEGER_TOO_LARGE => "IntegerTooLarge",
            Self::CODEC_DER_ERROR_REASON_TRUNCATED_INPUT => "TruncatedInput",
            Self::CODEC_DER_ERROR_REASON_INDEFINITE_LENGTH => "IndefiniteLength",
            Self::CODEC_DER_ERROR_REASON_NON_CANONICAL_ENCODING => "NonCanonicalEncoding",
            Self::CODEC_DER_ERROR_REASON_INVALID_INTEGER => "InvalidInteger",
            Self::CODEC_DER_ERROR_REASON_INVALID_TAG => "InvalidTag",
            Self::CODEC_DER_ERROR_REASON_UNEXPECTED_TAG => "UnexpectedTag",
            Self::CODEC_DER_ERROR_REASON_INVALID_LENGTH => "InvalidLength",
            Self::CODEC_DER_ERROR_REASON_INVALID_VALUE => "InvalidValue",
            Self::CODEC_DER_ERROR_REASON_TRAILING_DATA => "TrailingData",
            Self::CODEC_DER_ERROR_REASON_SET_ORDERING => "SetOrdering",
            Self::CODEC_DER_ERROR_REASON_ARITHMETIC_OVERFLOW => "ArithmeticOverflow",
            Self::CODEC_DER_ERROR_REASON_ALLOCATION_FAILURE => "AllocationFailure",
        };
        formatter.write_str(label)
    }
}

impl ::std::error::Error for CodecDerErrorReason {}

impl ::core::fmt::Display for CodecTlvErrorReason {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_TLV_ERROR_REASON_UNSPECIFIED => "unspecified error reason",
            Self::CODEC_TLV_ERROR_REASON_EMPTY_INPUT => "empty input",
            Self::CODEC_TLV_ERROR_REASON_INVALID_PROFILE => "unsupported framing profile",
            Self::CODEC_TLV_ERROR_REASON_INVALID_LIMITS => "invalid resource limits",
            Self::CODEC_TLV_ERROR_REASON_INPUT_LIMIT => "input limit exceeded",
            Self::CODEC_TLV_ERROR_REASON_IDENTIFIER_LIMIT => "identifier octet limit exceeded",
            Self::CODEC_TLV_ERROR_REASON_LENGTH_LIMIT => "length octet limit exceeded",
            Self::CODEC_TLV_ERROR_REASON_TRUNCATED_IDENTIFIER => "truncated identifier",
            Self::CODEC_TLV_ERROR_REASON_TRUNCATED_LENGTH => "truncated length",
            Self::CODEC_TLV_ERROR_REASON_TRUNCATED_VALUE => "truncated value",
            Self::CODEC_TLV_ERROR_REASON_UNSUPPORTED_INDEFINITE_LENGTH => {
                "indefinite length is not supported"
            }
            Self::CODEC_TLV_ERROR_REASON_NONCANONICAL_IDENTIFIER => "noncanonical identifier",
            Self::CODEC_TLV_ERROR_REASON_NONCANONICAL_LENGTH => "noncanonical length",
            Self::CODEC_TLV_ERROR_REASON_TRAILING_DATA => "trailing data",
            Self::CODEC_TLV_ERROR_REASON_ARITHMETIC_OVERFLOW => "length arithmetic overflow",
            Self::CODEC_TLV_ERROR_REASON_INVALID_LENGTH => "reserved length encoding",
            Self::CODEC_TLV_ERROR_REASON_INVALID_IDENTIFIER => "invalid definite-value identifier",
        };
        formatter.write_str(label)
    }
}

impl ::core::fmt::Debug for CodecTlvErrorReason {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_TLV_ERROR_REASON_UNSPECIFIED => "Unspecified",
            Self::CODEC_TLV_ERROR_REASON_EMPTY_INPUT => "EmptyInput",
            Self::CODEC_TLV_ERROR_REASON_INVALID_PROFILE => "InvalidProfile",
            Self::CODEC_TLV_ERROR_REASON_INVALID_LIMITS => "InvalidLimits",
            Self::CODEC_TLV_ERROR_REASON_INPUT_LIMIT => "InputLimit",
            Self::CODEC_TLV_ERROR_REASON_IDENTIFIER_LIMIT => "IdentifierLimit",
            Self::CODEC_TLV_ERROR_REASON_LENGTH_LIMIT => "LengthLimit",
            Self::CODEC_TLV_ERROR_REASON_TRUNCATED_IDENTIFIER => "TruncatedIdentifier",
            Self::CODEC_TLV_ERROR_REASON_TRUNCATED_LENGTH => "TruncatedLength",
            Self::CODEC_TLV_ERROR_REASON_TRUNCATED_VALUE => "TruncatedValue",
            Self::CODEC_TLV_ERROR_REASON_UNSUPPORTED_INDEFINITE_LENGTH => {
                "UnsupportedIndefiniteLength"
            }
            Self::CODEC_TLV_ERROR_REASON_NONCANONICAL_IDENTIFIER => "NoncanonicalIdentifier",
            Self::CODEC_TLV_ERROR_REASON_NONCANONICAL_LENGTH => "NoncanonicalLength",
            Self::CODEC_TLV_ERROR_REASON_TRAILING_DATA => "TrailingData",
            Self::CODEC_TLV_ERROR_REASON_ARITHMETIC_OVERFLOW => "ArithmeticOverflow",
            Self::CODEC_TLV_ERROR_REASON_INVALID_LENGTH => "InvalidLength",
            Self::CODEC_TLV_ERROR_REASON_INVALID_IDENTIFIER => "InvalidIdentifier",
        };
        formatter.write_str(label)
    }
}

impl ::std::error::Error for CodecTlvErrorReason {}

impl ::core::fmt::Debug for CodecGeneralCborLengthPolicy {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_GENERAL_CBOR_LENGTH_POLICY_UNSPECIFIED => "Unspecified",
            Self::CODEC_GENERAL_CBOR_LENGTH_POLICY_DEFINITE_ONLY => "DefiniteOnly",
            Self::CODEC_GENERAL_CBOR_LENGTH_POLICY_ALLOW_INDEFINITE => "AllowIndefinite",
        };
        formatter.write_str(label)
    }
}

impl ::core::fmt::Debug for CodecGeneralCborSerialization {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_GENERAL_CBOR_SERIALIZATION_UNSPECIFIED => "Unspecified",
            Self::CODEC_GENERAL_CBOR_SERIALIZATION_RECEIVED => "Received",
            Self::CODEC_GENERAL_CBOR_SERIALIZATION_PREFERRED => "Preferred",
        };
        formatter.write_str(label)
    }
}

impl ::core::fmt::Debug for CodecGeneralCborFloatPolicy {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_GENERAL_CBOR_FLOAT_POLICY_UNSPECIFIED => "Unspecified",
            Self::CODEC_GENERAL_CBOR_FLOAT_POLICY_ALL => "All",
            Self::CODEC_GENERAL_CBOR_FLOAT_POLICY_FINITE_ONLY => "FiniteOnly",
            Self::CODEC_GENERAL_CBOR_FLOAT_POLICY_REJECT => "Reject",
        };
        formatter.write_str(label)
    }
}

impl ::core::fmt::Debug for CodecGeneralCborMapOrdering {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_GENERAL_CBOR_MAP_ORDERING_UNSPECIFIED => "Unspecified",
            Self::CODEC_GENERAL_CBOR_MAP_ORDERING_RECEIVED => "Received",
            Self::CODEC_GENERAL_CBOR_MAP_ORDERING_LENGTH_FIRST => "LengthFirst",
            Self::CODEC_GENERAL_CBOR_MAP_ORDERING_BYTEWISE => "Bytewise",
        };
        formatter.write_str(label)
    }
}

impl ::core::fmt::Debug for CodecGeneralCborFloatWidth {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_GENERAL_CBOR_FLOAT_WIDTH_UNSPECIFIED => "Unspecified",
            Self::CODEC_GENERAL_CBOR_FLOAT_WIDTH_BINARY16 => "Binary16",
            Self::CODEC_GENERAL_CBOR_FLOAT_WIDTH_BINARY32 => "Binary32",
            Self::CODEC_GENERAL_CBOR_FLOAT_WIDTH_BINARY64 => "Binary64",
        };
        formatter.write_str(label)
    }
}

impl ::core::fmt::Debug for CodecGeneralCborMajorType {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_GENERAL_CBOR_MAJOR_TYPE_UNSPECIFIED => "Unspecified",
            Self::CODEC_GENERAL_CBOR_MAJOR_TYPE_UNSIGNED => "Unsigned",
            Self::CODEC_GENERAL_CBOR_MAJOR_TYPE_NEGATIVE => "Negative",
            Self::CODEC_GENERAL_CBOR_MAJOR_TYPE_BYTES => "Bytes",
            Self::CODEC_GENERAL_CBOR_MAJOR_TYPE_TEXT => "Text",
            Self::CODEC_GENERAL_CBOR_MAJOR_TYPE_ARRAY => "Array",
            Self::CODEC_GENERAL_CBOR_MAJOR_TYPE_MAP => "Map",
            Self::CODEC_GENERAL_CBOR_MAJOR_TYPE_TAG => "Tag",
            Self::CODEC_GENERAL_CBOR_MAJOR_TYPE_SIMPLE => "Simple",
        };
        formatter.write_str(label)
    }
}

impl ::core::fmt::Debug for CodecGeneralCborArgumentWidth {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let label = match self {
            Self::CODEC_GENERAL_CBOR_ARGUMENT_WIDTH_UNSPECIFIED => "Unspecified",
            Self::CODEC_GENERAL_CBOR_ARGUMENT_WIDTH_INLINE => "Inline",
            Self::CODEC_GENERAL_CBOR_ARGUMENT_WIDTH_ONE => "One",
            Self::CODEC_GENERAL_CBOR_ARGUMENT_WIDTH_TWO => "Two",
            Self::CODEC_GENERAL_CBOR_ARGUMENT_WIDTH_FOUR => "Four",
            Self::CODEC_GENERAL_CBOR_ARGUMENT_WIDTH_EIGHT => "Eight",
            Self::CODEC_GENERAL_CBOR_ARGUMENT_WIDTH_INDEFINITE => "Indefinite",
            Self::CODEC_GENERAL_CBOR_ARGUMENT_WIDTH_BREAK => "Break",
        };
        formatter.write_str(label)
    }
}
