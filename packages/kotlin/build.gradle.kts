// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import org.gradle.api.publish.maven.tasks.PublishToMavenLocal
import org.gradle.api.publish.maven.tasks.PublishToMavenRepository
import org.gradle.external.javadoc.StandardJavadocDocletOptions
import org.gradle.jvm.tasks.Jar
import java.net.URI
import java.security.MessageDigest
import java.util.zip.ZipFile

plugins {
    kotlin("jvm") version "2.4.0"
    `java-library`
    `maven-publish`
    signing
}

group = "me.really"
version = "0.2.0"

dependencyLocking {
    lockAllConfigurations()
}

val remoteMavenRepositoryUrl = providers.gradleProperty("reallyme.maven.repositoryUrl")
    .orElse(providers.environmentVariable("REALLYME_MAVEN_REPOSITORY_URL"))
val remoteMavenUsername = providers.gradleProperty("reallyme.maven.username")
    .orElse(providers.environmentVariable("REALLYME_MAVEN_USERNAME"))
val remoteMavenPassword = providers.gradleProperty("reallyme.maven.password")
    .orElse(providers.environmentVariable("REALLYME_MAVEN_PASSWORD"))
val signingKey = providers.gradleProperty("signingInMemoryKey")
    .orElse(providers.environmentVariable("MAVEN_SIGNING_KEY"))
val signingPassword = providers.gradleProperty("signingInMemoryKeyPassword")
    .orElse(providers.environmentVariable("MAVEN_SIGNING_PASSWORD"))
fun nonBlank(value: String?): String? = value?.trim()?.takeIf { it.isNotEmpty() }

val remoteMavenRepositoryUrlValue = nonBlank(remoteMavenRepositoryUrl.orNull)
val remoteMavenUsernameValue = nonBlank(remoteMavenUsername.orNull)
val remoteMavenPasswordValue = nonBlank(remoteMavenPassword.orNull)
val signingKeyValue = nonBlank(signingKey.orNull)
val signingPasswordValue = nonBlank(signingPassword.orNull)
val remoteMavenRepositoryUri = remoteMavenRepositoryUrlValue?.let { value ->
    val parsed = try {
        URI(value)
    } catch (_: IllegalArgumentException) {
        throw GradleException("remote Maven repository URL is invalid")
    }
    if (
        parsed.scheme != "https" ||
        parsed.host.isNullOrBlank() ||
        parsed.userInfo != null ||
        parsed.query != null ||
        parsed.fragment != null
    ) {
        throw GradleException(
            "remote Maven repository URL must be an absolute HTTPS URL without embedded credentials, a query, or a fragment"
        )
    }
    parsed
}
val configuredNativeResourcesDir = providers.gradleProperty("reallyme.codec.nativeResourcesDir")
val nativeResourcesDir = configuredNativeResourcesDir
    .map { file(it) }
    .orElse(layout.buildDirectory.dir("generated/native-resources").map { it.asFile })
val requireFullNativeResources = providers.gradleProperty("reallyme.codec.requireFullNativeResources")
    .map { it == "true" }
    .orElse(false)
val requiredNativeResources = listOf(
    "me/really/codec/native/linux-x86_64/libreallyme_codec_ffi.so",
    "me/really/codec/native/linux-x86_64/libreallyme_codec_ffi.so.sha256",
    "me/really/codec/native/linux-aarch64/libreallyme_codec_ffi.so",
    "me/really/codec/native/linux-aarch64/libreallyme_codec_ffi.so.sha256",
    "me/really/codec/native/macos-x86_64/libreallyme_codec_ffi.dylib",
    "me/really/codec/native/macos-x86_64/libreallyme_codec_ffi.dylib.sha256",
    "me/really/codec/native/macos-aarch64/libreallyme_codec_ffi.dylib",
    "me/really/codec/native/macos-aarch64/libreallyme_codec_ffi.dylib.sha256",
    "me/really/codec/native/windows-x86_64/reallyme_codec_ffi.dll",
    "me/really/codec/native/windows-x86_64/reallyme_codec_ffi.dll.sha256",
    "me/really/codec/native/native-manifest.json",
)
val hostNativePlatform = when {
    System.getProperty("os.name").contains("Mac", ignoreCase = true) -> "macos"
    System.getProperty("os.name").contains("Linux", ignoreCase = true) -> "linux"
    System.getProperty("os.name").contains("Windows", ignoreCase = true) -> "windows"
    else -> throw GradleException("unsupported host operating system for ReallyMe codec JNI resources")
}
val hostNativeArch = when (System.getProperty("os.arch").lowercase()) {
    "aarch64", "arm64" -> "aarch64"
    "amd64", "x86_64" -> "x86_64"
    else -> throw GradleException("unsupported host architecture for ReallyMe codec JNI resources")
}
val hostNativeLibraryName = when (hostNativePlatform) {
    "macos" -> "libreallyme_codec_ffi.dylib"
    "windows" -> "reallyme_codec_ffi.dll"
    else -> "libreallyme_codec_ffi.so"
}
val requiredHostNativeResource =
    "me/really/codec/native/$hostNativePlatform-$hostNativeArch/$hostNativeLibraryName"
val requiredHostNativeDigest = "$requiredHostNativeResource.sha256"
val ffiRustFlags = listOfNotNull(
    nonBlank(providers.environmentVariable("RUSTFLAGS").orNull),
    "-C panic=unwind",
).joinToString(" ")

fun sha256Hex(file: File): String {
    val digest = MessageDigest.getInstance("SHA-256")
    file.inputStream().use { input ->
        val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
        try {
            while (true) {
                val read = input.read(buffer)
                if (read < 0) {
                    break
                }
                if (read > 0) {
                    digest.update(buffer, 0, read)
                }
            }
        } finally {
            buffer.fill(0)
        }
    }
    return digest.digest().joinToString(separator = "") { byte ->
        "%02x".format(byte)
    }
}

fun sha256Hex(bytes: ByteArray): String {
    val digest = MessageDigest.getInstance("SHA-256").digest(bytes)
    return digest.joinToString(separator = "") { byte ->
        "%02x".format(byte)
    }
}

fun expectedNativeDigestMetadata(file: File): String =
    "${sha256Hex(file)} ${file.length()}\n"

fun expectedNativeDigestMetadata(bytes: ByteArray): String =
    "${sha256Hex(bytes)} ${bytes.size}\n"

kotlin {
    jvmToolchain(21)
    sourceSets {
        main {
            kotlin.srcDir("../../gen/kotlin")
        }
    }
}

java {
    withSourcesJar()
    withJavadocJar()
}

sourceSets {
    named("main") {
        java.srcDir("../../gen/java")
        resources.srcDir(nativeResourcesDir)
    }
}

val buildHostNativeLibrary = tasks.register<Exec>("buildHostNativeLibrary") {
    group = "build"
    description = "Builds the host Rust JNI library for local JVM tests."
    onlyIf { !configuredNativeResourcesDir.isPresent }
    workingDir = layout.projectDirectory.dir("../..").asFile
    environment("RUSTFLAGS", ffiRustFlags)
    commandLine("cargo", "build", "--locked", "-p", "reallyme-codec-ffi", "--release")
}

val stageHostNativeResource = tasks.register<Copy>("stageHostNativeResource") {
    group = "build"
    description = "Stages the host Rust JNI library as a JVM package resource for local tests."
    onlyIf { !configuredNativeResourcesDir.isPresent }
    dependsOn(buildHostNativeLibrary)
    from(layout.projectDirectory.file("../../target/release/$hostNativeLibraryName"))
    into(nativeResourcesDir.map {
        it.resolve("me/really/codec/native/$hostNativePlatform-$hostNativeArch")
    })
}

val writeHostNativeDigest = tasks.register("writeHostNativeDigest") {
    group = "build"
    description = "Writes bounded integrity metadata for the local host JNI library."
    onlyIf { !configuredNativeResourcesDir.isPresent }
    dependsOn(stageHostNativeResource)
    val library = nativeResourcesDir.map { it.resolve(requiredHostNativeResource) }
    val sidecar = nativeResourcesDir.map { it.resolve(requiredHostNativeDigest) }
    inputs.file(library)
    outputs.file(sidecar)
    doLast {
        val libraryFile = library.get()
        val sidecarFile = sidecar.get()
        sidecarFile.parentFile.mkdirs()
        sidecarFile.writeText(expectedNativeDigestMetadata(libraryFile))
    }
}

dependencies {
    api("com.google.protobuf:protobuf-javalite:4.35.1")
    api("com.google.protobuf:protobuf-kotlin-lite:4.35.1")
    testImplementation("com.google.code.gson:gson:2.11.0")
    testImplementation("org.junit.jupiter:junit-jupiter-api:5.11.4")
    testImplementation(kotlin("test"))
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.11.4")
}

tasks.test {
    useJUnitPlatform()
    providers.environmentVariable("REALLYME_CODEC_FFI_LIBRARY_PATH").orNull?.let { libraryPath ->
        systemProperty("reallyme.codec.testLibraryPath", libraryPath)
    }
}

tasks.named("processResources") {
    dependsOn(writeHostNativeDigest)
}

tasks.named("sourcesJar") {
    dependsOn(writeHostNativeDigest)
}

tasks.withType<Javadoc>().configureEach {
    val standardOptions = options as StandardJavadocDocletOptions
    standardOptions.addStringOption("Xdoclint:none", "-quiet")
}

val verifyBundledNativeResources = tasks.register("verifyBundledNativeResources") {
    group = "verification"
    description = "Verifies that release JVM artifacts include every supported native FFI library."
    inputs.dir(nativeResourcesDir)
    doLast {
        val root = nativeResourcesDir.get()
        val missing = requiredNativeResources.filter { relativePath ->
            !root.resolve(relativePath).isFile
        }
        if (missing.isNotEmpty()) {
            throw GradleException(
                "missing ReallyMe codec native resources: ${missing.joinToString(", ")}"
            )
        }
        for (relativePath in requiredNativeResources.filter { it.endsWith(".so") || it.endsWith(".dylib") || it.endsWith(".dll") }) {
            val library = root.resolve(relativePath)
            val sidecar = root.resolve("$relativePath.sha256")
            val expected = expectedNativeDigestMetadata(library)
            if (sidecar.readText() != expected) {
                throw GradleException(
                    "ReallyMe codec native digest does not match $relativePath"
                )
            }
        }
    }
}

val verifyHostBundledNativeResource = tasks.register("verifyHostBundledNativeResource") {
    group = "verification"
    description = "Verifies that local JVM artifacts include the host Rust JNI library."
    dependsOn(writeHostNativeDigest)
    inputs.dir(nativeResourcesDir)
    doLast {
        val root = nativeResourcesDir.get()
        if (!root.resolve(requiredHostNativeResource).isFile) {
            throw GradleException(
                "missing ReallyMe codec host native resource: $requiredHostNativeResource"
            )
        }
        if (!root.resolve(requiredHostNativeDigest).isFile) {
            throw GradleException(
                "missing ReallyMe codec host native digest: $requiredHostNativeDigest"
            )
        }
        if (
            root.resolve(requiredHostNativeDigest).readText() !=
            expectedNativeDigestMetadata(root.resolve(requiredHostNativeResource))
        ) {
            throw GradleException(
                "ReallyMe codec host native digest does not match $requiredHostNativeResource"
            )
        }
    }
}

val verifyJarContainsNativeResources = tasks.register("verifyJarContainsNativeResources") {
    group = "verification"
    description = "Verifies that the packaged JVM JAR contains native resources with matching digests."
    val jarTask = tasks.named<Jar>("jar")
    dependsOn(jarTask)
    if (requireFullNativeResources.get()) {
        dependsOn(verifyBundledNativeResources)
    } else {
        dependsOn(verifyHostBundledNativeResource)
    }
    inputs.file(jarTask.flatMap { it.archiveFile })
    doLast {
        val requiredResources = if (requireFullNativeResources.get()) {
            requiredNativeResources
        } else {
            listOf(requiredHostNativeResource, requiredHostNativeDigest)
        }
        ZipFile(jarTask.get().archiveFile.get().asFile).use { archive ->
            for (relativePath in requiredResources) {
                archive.getEntry(relativePath)
                    ?: throw GradleException("JVM JAR is missing native resource: $relativePath")
            }
            val nativeLibraries = requiredResources.filter {
                it.endsWith(".so") || it.endsWith(".dylib") || it.endsWith(".dll")
            }
            for (relativePath in nativeLibraries) {
                val libraryEntry = archive.getEntry(relativePath)
                    ?: throw GradleException("JVM JAR is missing native library: $relativePath")
                val sidecarEntry = archive.getEntry("$relativePath.sha256")
                    ?: throw GradleException("JVM JAR is missing native digest: $relativePath.sha256")
                val bytes = archive.getInputStream(libraryEntry).use { it.readBytes() }
                try {
                    val sidecarText = archive.getInputStream(sidecarEntry)
                        .bufferedReader(Charsets.US_ASCII)
                        .use { it.readText() }
                    if (sidecarText != expectedNativeDigestMetadata(bytes)) {
                        throw GradleException("JVM JAR native digest does not match $relativePath")
                    }
                } finally {
                    bytes.fill(0)
                }
            }
        }
    }
}

tasks.withType<PublishToMavenLocal>().configureEach {
    dependsOn(verifyJarContainsNativeResources)
    if (requireFullNativeResources.get()) {
        dependsOn(verifyBundledNativeResources)
    }
}

tasks.withType<PublishToMavenRepository>().configureEach {
    dependsOn(verifyBundledNativeResources, verifyJarContainsNativeResources)
}

val verifyRemoteMavenPublishingConfigured = tasks.register("verifyRemoteMavenPublishingConfigured") {
    group = "verification"
    description = "Verifies that remote Maven publishing credentials are configured."
    doLast {
        val missing = buildList {
            if (remoteMavenRepositoryUrlValue == null) {
                add("REALLYME_MAVEN_REPOSITORY_URL or -Preallyme.maven.repositoryUrl")
            }
            if (remoteMavenUsernameValue == null) {
                add("REALLYME_MAVEN_USERNAME or -Preallyme.maven.username")
            }
            if (remoteMavenPasswordValue == null) {
                add("REALLYME_MAVEN_PASSWORD or -Preallyme.maven.password")
            }
            if (signingKeyValue == null) {
                add("MAVEN_SIGNING_KEY or -PsigningInMemoryKey")
            }
            if (signingPasswordValue == null) {
                add("MAVEN_SIGNING_PASSWORD or -PsigningInMemoryKeyPassword")
            }
        }
        if (missing.isNotEmpty()) {
            throw GradleException(
                "remote Maven publishing is not configured; missing non-empty ${missing.joinToString(", ")}"
            )
        }
    }
}

tasks.named("publish") {
    dependsOn(verifyRemoteMavenPublishingConfigured)
}

tasks.withType<PublishToMavenRepository>().configureEach {
    dependsOn(verifyRemoteMavenPublishingConfigured)
}

publishing {
    publications {
        create<MavenPublication>("maven") {
            artifactId = "codec"
            from(components["java"])
            pom {
                name.set("ReallyMe Codec")
                description.set("ReallyMe codec compatibility facade for Java, Kotlin, JVM, and Android.")
                url.set("https://github.com/reallyme/codec")
                licenses {
                    license {
                        name.set("Apache License, Version 2.0")
                        url.set("https://www.apache.org/licenses/LICENSE-2.0.txt")
                        distribution.set("repo")
                    }
                }
                developers {
                    developer {
                        id.set("reallyme")
                        name.set("ReallyMe LLC")
                        organization.set("ReallyMe LLC")
                        organizationUrl.set("https://github.com/reallyme")
                    }
                }
                scm {
                    connection.set("scm:git:https://github.com/reallyme/codec.git")
                    developerConnection.set("scm:git:ssh://git@github.com/reallyme/codec.git")
                    url.set("https://github.com/reallyme/codec")
                }
            }
        }
    }
    repositories {
        if (remoteMavenRepositoryUri != null) {
            maven {
                name = "remoteRelease"
                url = remoteMavenRepositoryUri
                credentials {
                    username = remoteMavenUsernameValue
                    password = remoteMavenPasswordValue
                }
            }
        }
    }
}

signing {
    if (signingKeyValue != null) {
        useInMemoryPgpKeys(signingKeyValue, signingPasswordValue)
        sign(publishing.publications["maven"])
    }
}
