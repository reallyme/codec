// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

public sealed class ReallyMeDeterministicCborInteger {
    public class Unsigned public constructor(public val value: ULong) :
        ReallyMeDeterministicCborInteger() {
        override fun toString(): String = "ReallyMeDeterministicCborInteger(<redacted>)"
    }

    public class Negative private constructor(public val value: Long) :
        ReallyMeDeterministicCborInteger() {
        public companion object {
            public fun of(value: Long): Negative {
                if (value >= 0) {
                    throw ReallyMeCodecException.InvalidInput()
                }
                return Negative(value)
            }

            internal fun fromProvider(value: Long): Negative {
                if (value >= 0) {
                    throw ReallyMeCodecException.ProviderFailure()
                }
                return Negative(value)
            }
        }

        override fun toString(): String = "ReallyMeDeterministicCborInteger(<redacted>)"
    }
}

public sealed class ReallyMeDeterministicCborMapKey {
    public class Integer public constructor(public val value: ReallyMeDeterministicCborInteger) :
        ReallyMeDeterministicCborMapKey() {
        override fun toString(): String = "ReallyMeDeterministicCborMapKey(<redacted>)"
    }

    public class Text public constructor(public val value: String) :
        ReallyMeDeterministicCborMapKey() {
        override fun toString(): String = "ReallyMeDeterministicCborMapKey(<redacted>)"
    }
}

public class ReallyMeDeterministicCborMapEntry public constructor(
    public val key: ReallyMeDeterministicCborMapKey,
    public val value: ReallyMeDeterministicCborValue,
) {
    override fun toString(): String = "ReallyMeDeterministicCborMapEntry(<redacted>)"
}

public object ReallyMeDeterministicCbor {
    @JvmStatic
    public fun nullValue(): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Null

    @JvmStatic
    public fun bool(value: Boolean): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Bool(value)

    @JvmStatic
    public fun unsigned(value: ULong): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Integer(
            ReallyMeDeterministicCborInteger.Unsigned(value),
        )

    @JvmStatic
    public fun unsignedLong(value: Long): ReallyMeDeterministicCborValue {
        if (value < 0) {
            throw ReallyMeCodecException.InvalidInput()
        }
        return unsigned(value.toULong())
    }

    @JvmStatic
    public fun negative(value: Long): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Integer(
            ReallyMeDeterministicCborInteger.Negative.of(value),
        )

    @JvmStatic
    public fun text(value: String): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Text(value)

    @JvmStatic
    public fun bytes(value: ByteArray): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Bytes(value)

    @JvmStatic
    public fun array(
        values: List<ReallyMeDeterministicCborValue>,
    ): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Array(values)

    @JvmStatic
    public fun mapInt(
        entries: List<Pair<ULong, ReallyMeDeterministicCborValue>>,
    ): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Map(
            entries.map { entry ->
                ReallyMeDeterministicCborMapEntry(
                    key = ReallyMeDeterministicCborMapKey.Integer(
                        ReallyMeDeterministicCborInteger.Unsigned(entry.first),
                    ),
                    value = entry.second,
                )
            },
        )

    @JvmStatic
    public fun mapUnsignedLong(
        entries: Map<Long, ReallyMeDeterministicCborValue>,
    ): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Map(
            entries.entries.map { entry ->
                val key = entry.key
                if (key < 0) {
                    throw ReallyMeCodecException.InvalidInput()
                }
                ReallyMeDeterministicCborMapEntry(
                    key = ReallyMeDeterministicCborMapKey.Integer(
                        ReallyMeDeterministicCborInteger.Unsigned(key.toULong()),
                    ),
                    value = entry.value,
                )
            },
        )

    @JvmStatic
    public fun mapText(
        entries: List<Pair<String, ReallyMeDeterministicCborValue>>,
    ): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Map(
            entries.map { entry ->
                ReallyMeDeterministicCborMapEntry(
                    key = ReallyMeDeterministicCborMapKey.Text(entry.first),
                    value = entry.second,
                )
            },
        )

    @JvmStatic
    public fun mapText(
        entries: Map<String, ReallyMeDeterministicCborValue>,
    ): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCborValue.Map(
            entries.entries.map { entry ->
                ReallyMeDeterministicCborMapEntry(
                    key = ReallyMeDeterministicCborMapKey.Text(entry.key),
                    value = entry.value,
                )
            },
        )

    @JvmStatic
    public fun intKey(value: ULong): ReallyMeDeterministicCborMapKey =
        ReallyMeDeterministicCborMapKey.Integer(
            ReallyMeDeterministicCborInteger.Unsigned(value),
        )

    @JvmStatic
    public fun intKey(value: Long): ReallyMeDeterministicCborMapKey =
        if (value >= 0) {
            ReallyMeDeterministicCborMapKey.Integer(
                ReallyMeDeterministicCborInteger.Unsigned(value.toULong()),
            )
        } else {
            ReallyMeDeterministicCborMapKey.Integer(
                ReallyMeDeterministicCborInteger.Negative.of(value),
            )
        }

    @JvmStatic
    public fun textKey(value: String): ReallyMeDeterministicCborMapKey =
        ReallyMeDeterministicCborMapKey.Text(value)

    @JvmStatic
    public fun entry(
        key: ReallyMeDeterministicCborMapKey,
        value: ReallyMeDeterministicCborValue,
    ): ReallyMeDeterministicCborMapEntry =
        ReallyMeDeterministicCborMapEntry(key, value)
}

public object ReallyMeDagCbor {
    @JvmStatic
    public fun nullValue(): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCbor.nullValue()

    @JvmStatic
    public fun bool(value: Boolean): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCbor.bool(value)

    @JvmStatic
    public fun unsigned(value: ULong): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCbor.unsigned(value)

    @JvmStatic
    public fun unsignedLong(value: Long): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCbor.unsignedLong(value)

    @JvmStatic
    public fun negative(value: Long): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCbor.negative(value)

    @JvmStatic
    public fun text(value: String): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCbor.text(value)

    @JvmStatic
    public fun bytes(value: ByteArray): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCbor.bytes(value)

    @JvmStatic
    public fun array(
        values: List<ReallyMeDeterministicCborValue>,
    ): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCbor.array(values)

    @JvmStatic
    public fun mapText(
        entries: List<Pair<String, ReallyMeDeterministicCborValue>>,
    ): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCbor.mapText(entries)

    @JvmStatic
    public fun mapText(
        entries: Map<String, ReallyMeDeterministicCborValue>,
    ): ReallyMeDeterministicCborValue =
        ReallyMeDeterministicCbor.mapText(entries)
}

public sealed class ReallyMeDeterministicCborValue {
    public data object Null : ReallyMeDeterministicCborValue() {
        override fun toString(): String = "ReallyMeDeterministicCborValue(<redacted>)"
    }

    public class Bool public constructor(public val value: Boolean) :
        ReallyMeDeterministicCborValue() {
        override fun toString(): String = "ReallyMeDeterministicCborValue(<redacted>)"
    }

    public class Integer public constructor(public val value: ReallyMeDeterministicCborInteger) :
        ReallyMeDeterministicCborValue() {
        override fun toString(): String = "ReallyMeDeterministicCborValue(<redacted>)"
    }

    public class Text public constructor(public val value: String) :
        ReallyMeDeterministicCborValue() {
        override fun toString(): String = "ReallyMeDeterministicCborValue(<redacted>)"
    }

    public class Bytes private constructor(
        private val bytes: ByteArray,
        @Suppress("UNUSED_PARAMETER") owned: Unit,
    ) : ReallyMeDeterministicCborValue() {
        public constructor(bytes: ByteArray) : this(bytes.copyOf(), Unit)

        internal companion object {
            /**
             * Transfers one freshly allocated provider-result array into the
             * public value owner. This avoids copying the bytes only to leave
             * the short-lived source array unwipeable until garbage collection.
             */
            internal fun fromOwnedProviderBytes(bytes: ByteArray): Bytes = Bytes(bytes, Unit)
        }

        public fun bytes(): ByteArray = bytes.copyOf()

        internal fun borrowedBytes(): ByteArray = bytes

        override fun toString(): String = "ReallyMeDeterministicCborValue(<redacted>)"
    }

    public class Array public constructor(
        values: List<ReallyMeDeterministicCborValue>,
    ) : ReallyMeDeterministicCborValue() {
        public val values: List<ReallyMeDeterministicCborValue> = values.toList()

        override fun toString(): String = "ReallyMeDeterministicCborValue(<redacted>)"
    }

    public class Map public constructor(
        entries: List<ReallyMeDeterministicCborMapEntry>,
    ) : ReallyMeDeterministicCborValue() {
        public val entries: List<ReallyMeDeterministicCborMapEntry> = entries.toList()

        override fun toString(): String = "ReallyMeDeterministicCborValue(<redacted>)"
    }
}
