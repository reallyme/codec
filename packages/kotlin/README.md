# ReallyMe Codec for Java and Kotlin

The `me.really:codec` JVM package includes the Kotlin/Java API and ships Rust
JNI libraries as platform resources. The matching library loads on first use.

```kotlin
dependencies {
    implementation("me.really:codec:0.4.5")
}
```

## Usage

```kotlin
import me.really.codec.ReallyMeCodec

val encoded = ReallyMeCodec.base64urlEncode(byteArrayOf(1, 2, 3))
val decoded = ReallyMeCodec.base64urlDecode(encoded)
```

Java callers use the same methods:

```java
import me.really.codec.ReallyMeCodec;

String encoded = ReallyMeCodec.base64urlEncode(new byte[] {1, 2, 3});
byte[] decoded = ReallyMeCodec.base64urlDecode(encoded);
```

Typed CBOR values use `ReallyMeDeterministicCbor` and `ReallyMeDagCbor` builders:

```kotlin
import me.really.codec.ReallyMeDeterministicCbor

val value = ReallyMeDeterministicCbor.mapText(
    listOf("b" to ReallyMeDeterministicCbor.unsignedLong(2)),
)
val encodedCbor = ReallyMeCodec.deterministicCborEncode(value)
```

See [format limits](../../README.md#formats-and-limits) for CBOR, CID and JCS
restrictions. Clear sensitive `ByteArray` values after use; immutable strings,
protobuf storage and other managed copies cannot be reliably erased.

`ReallyMeCodecException.reason` and `.origin` expose generated protobuf error
values. They are null when no validated error envelope was received.
`processOperation` and `processOperationJson` accept serialized requests and
return binary protobuf responses; see the [protobuf API](../../docs/protobuf.md).

## Platforms

Requires Java 21 or newer.

Linux x86-64 and AArch64 libraries require glibc 2.34 or newer. They do not
support musl systems such as Alpine Linux. Unsupported native environments
return `ProviderFailure`. Use [codec-android](../kotlin-android/README.md) for
Android applications.

## Test

From the repository root:

```sh
packages/kotlin/gradlew -p packages/kotlin test
```

Host tests build and stage the native library automatically. To test an
explicit resource tree:

```sh
scripts/build_kotlin_native_resource.sh build/kotlin-native-resources
packages/kotlin/gradlew -p packages/kotlin test \
  -Preallyme.codec.nativeResourcesDir="$PWD/build/kotlin-native-resources"
```

## License

MIT or Apache-2.0, at your option. See [MIT](../../LICENSE-MIT) and
[Apache 2.0](../../LICENSE-APACHE).
