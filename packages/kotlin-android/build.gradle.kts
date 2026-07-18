// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import org.gradle.api.publish.maven.tasks.PublishToMavenLocal
import org.gradle.api.publish.maven.tasks.PublishToMavenRepository
import java.net.URI
import java.security.MessageDigest
import java.util.zip.ZipFile

plugins {
    id("com.android.library") version "9.3.0"
    id("com.android.application") version "9.3.0" apply false
    `maven-publish`
    signing
}

group = "me.really"
version = "0.2.0"

dependencyLocking {
    lockAllConfigurations()
}

val configuredAndroidJniLibsDir = providers.gradleProperty("reallyme.codec.androidJniLibsDir")
    .map { file(it) }
val jniLibsDir = configuredAndroidJniLibsDir
    .orElse(layout.buildDirectory.dir("generated/android-jniLibs").map { it.asFile })
val configuredNativeAssetsDir = providers.gradleProperty("reallyme.codec.androidNativeAssetsDir")
val nativeAssetsDir = configuredNativeAssetsDir
    .map { file(it) }
    .orElse(layout.buildDirectory.dir("generated/android-native-assets").map { it.asFile })
val requireJniLibs = providers.gradleProperty("reallyme.codec.requireAndroidJniLibs")
    .map { it == "true" }
    .orElse(false)
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

val requiredAndroidJniLibs = listOf(
    "arm64-v8a/libreallyme_codec_ffi.so",
    "armeabi-v7a/libreallyme_codec_ffi.so",
    "x86_64/libreallyme_codec_ffi.so",
    "x86/libreallyme_codec_ffi.so",
)
val requiredAndroidNativeManifest = "reallyme-codec/native-manifest.json"

fun sha256Hex(bytes: ByteArray): String {
    val digest = MessageDigest.getInstance("SHA-256").digest(bytes)
    return digest.joinToString(separator = "") { byte -> "%02x".format(byte) }
}

fun checkedOutCommitSha(): String {
    val checkedOutSha = providers.exec {
        workingDir = layout.projectDirectory.dir("../..").asFile
        commandLine("git", "rev-parse", "HEAD")
    }.standardOutput.asText.get().trim()
    val fullSha = Regex("^[0-9a-f]{40}$")
    if (!fullSha.matches(checkedOutSha)) {
        throw GradleException("checked-out git commit SHA is not a lowercase full SHA")
    }
    val githubSha = providers.environmentVariable("GITHUB_SHA").orNull
    if (githubSha != null) {
        if (!fullSha.matches(githubSha)) {
            throw GradleException("GITHUB_SHA is not a lowercase full SHA")
        }
        if (githubSha != checkedOutSha) {
            throw GradleException("GITHUB_SHA does not match the checked-out source SHA")
        }
    }
    return checkedOutSha
}

fun verifyAndroidNativeManifestEntry(
    manifestText: String,
    relativePath: String,
    bytes: ByteArray,
) {
    val expectedDigest = sha256Hex(bytes)
    val pattern = Regex(
        """\{[^{}]*"path"\s*:\s*"${Regex.escape(relativePath)}"[^{}]*"sha256"\s*:\s*"$expectedDigest"[^{}]*"size"\s*:\s*${bytes.size}[^{}]*}""",
    )
    if (!pattern.containsMatchIn(manifestText)) {
        throw GradleException(
            "Android native manifest digest does not match $relativePath"
        )
    }
}

android {
    namespace = "me.really.codec"
    compileSdk = 36

    defaultConfig {
        minSdk = 24
        consumerProguardFiles("consumer-rules.pro")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }

    sourceSets {
        named("main") {
            java.directories.clear()
            java.directories.add("../kotlin/src/main/kotlin")
            java.directories.add("../../gen/java")
            java.directories.add("../../gen/kotlin")
            kotlin.directories.clear()
            kotlin.directories.add("../kotlin/src/main/kotlin")
            kotlin.directories.add("../../gen/kotlin")
            jniLibs.directories.clear()
            jniLibs.directories.add(jniLibsDir.get().path)
            assets.directories.add(nativeAssetsDir.get().path)
        }
    }

    packaging {
        jniLibs {
            // The release workflows hash the staged libraries before building
            // the AAR. Keep Gradle from mutating those bytes after attestation.
            keepDebugSymbols.add("**/libreallyme_codec_ffi.so")
        }
    }

    publishing {
        singleVariant("release") {
            withSourcesJar()
        }
    }
}

dependencies {
    api("com.google.protobuf:protobuf-javalite:4.35.1")
    api("com.google.protobuf:protobuf-kotlin-lite:4.35.1")
}

val generateAndroidNativeManifest = tasks.register("generateAndroidNativeManifest") {
    group = "build"
    description = "Generates the Android native checksum manifest for local AAR builds."
    onlyIf {
        !configuredNativeAssetsDir.isPresent &&
            (requireJniLibs.get() || jniLibsDir.get().isDirectory)
    }
    inputs.dir(jniLibsDir).optional()
    outputs.file(nativeAssetsDir.map { it.resolve(requiredAndroidNativeManifest) })
    doLast {
        val root = jniLibsDir.get()
        val nativeFiles = requiredAndroidJniLibs.map { relativePath ->
            val file = root.resolve(relativePath)
            if (!file.isFile) {
                throw GradleException(
                    "missing freshly built ReallyMe codec Android jniLib for manifest: $relativePath; " +
                        "run scripts/build_android_native_resources.sh and pass " +
                        "-Preallyme.codec.androidJniLibsDir"
                )
            }
            file
        }
        val commitSha = checkedOutCommitSha()
        val entries = nativeFiles.map { file ->
            val relativePath = root.toPath().relativize(file.toPath()).toString().replace(File.separatorChar, '/')
            val bytes = file.readBytes()
            """{"path":"$relativePath","sha256":"${sha256Hex(bytes)}","size":${bytes.size}}"""
        }.joinToString(",")
        val manifest = """
            {"schemaVersion":1,"package":"reallyme-codec-native","commitSha":"$commitSha","entries":[$entries]}
        """.trimIndent() + "\n"
        val manifestFile = nativeAssetsDir.get().resolve(requiredAndroidNativeManifest)
        manifestFile.parentFile.mkdirs()
        manifestFile.writeText(manifest)
    }
}

val verifyAndroidJniLibs = tasks.register("verifyAndroidJniLibs") {
    group = "verification"
    description = "Verifies that release Android AARs include every supported Rust JNI library."
    dependsOn(generateAndroidNativeManifest)
    inputs.dir(jniLibsDir).optional()
    inputs.dir(nativeAssetsDir).optional()
    onlyIf { requireJniLibs.get() }
    doLast {
        val root = jniLibsDir.get()
        val assetsRoot = nativeAssetsDir.get()
        val missing = requiredAndroidJniLibs.filter { relativePath ->
            !root.resolve(relativePath).isFile
        }
        if (missing.isNotEmpty()) {
            throw GradleException(
                "missing ReallyMe codec Android jniLibs: ${missing.joinToString(", ")}"
            )
        }
        if (!assetsRoot.resolve(requiredAndroidNativeManifest).isFile) {
            throw GradleException(
                "missing ReallyMe codec Android native manifest: $requiredAndroidNativeManifest"
            )
        }
        val manifestText = assetsRoot.resolve(requiredAndroidNativeManifest).readText()
        for (relativePath in requiredAndroidJniLibs) {
            val bytes = root.resolve(relativePath).readBytes()
            try {
                verifyAndroidNativeManifestEntry(manifestText, relativePath, bytes)
            } finally {
                bytes.fill(0)
            }
        }
    }
}

tasks.named("preBuild") {
    dependsOn(generateAndroidNativeManifest, verifyAndroidJniLibs)
}

tasks.register("verifyReleaseAarContainsJniLibs") {
    group = "verification"
    description = "Verifies that the release AAR contains the expected jniLibs entries."
    dependsOn(generateAndroidNativeManifest, "bundleReleaseAar")
    doLast {
        val aarFiles = layout.buildDirectory.dir("outputs/aar").get().asFile
            .listFiles { file -> file.isFile && file.name.endsWith("-release.aar") }
            ?.toList()
            .orEmpty()
        if (aarFiles.size != 1) {
            throw GradleException(
                "expected exactly one release AAR, found ${aarFiles.size}"
            )
        }
        ZipFile(aarFiles.single()).use { archive ->
            val manifestEntry = archive.getEntry("assets/$requiredAndroidNativeManifest")
                ?: throw GradleException(
                    "release AAR is missing native manifest asset: $requiredAndroidNativeManifest"
                )
            val manifestText = archive.getInputStream(manifestEntry)
                .bufferedReader()
                .use { it.readText() }
            for (relativePath in requiredAndroidJniLibs) {
                val entry = archive.getEntry("jni/$relativePath")
                    ?: throw GradleException("release AAR is missing JNI entry: $relativePath")
                val bytes = archive.getInputStream(entry).use { it.readBytes() }
                try {
                    verifyAndroidNativeManifestEntry(manifestText, relativePath, bytes)
                } finally {
                    bytes.fill(0)
                }
            }
        }
    }
}

tasks.withType<PublishToMavenLocal>().configureEach {
    dependsOn("verifyReleaseAarContainsJniLibs")
}

tasks.withType<PublishToMavenRepository>().configureEach {
    dependsOn("verifyReleaseAarContainsJniLibs")
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
        create<MavenPublication>("release") {
            artifactId = "codec-android"
            afterEvaluate {
                from(components["release"])
            }
            pom {
                name.set("ReallyMe Codec Android")
                description.set("ReallyMe codec Android facade backed by bundled Rust JNI libraries.")
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
        sign(publishing.publications["release"])
    }
}
