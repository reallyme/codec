// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import org.gradle.api.publish.maven.tasks.PublishToMavenLocal
import org.gradle.api.publish.maven.tasks.PublishToMavenRepository
import java.security.MessageDigest

plugins {
    id("com.android.library") version "9.3.0"
    `maven-publish`
    signing
}

group = "me.really"
version = "0.1.20"

val jniLibsDir = providers.gradleProperty("reallyme.codec.androidJniLibsDir")
    .map { file(it) }
    .orElse(layout.projectDirectory.dir("src/main/jniLibs").asFile)
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
            java.directories.add("../kotlin-codec/src/main/kotlin")
            java.directories.add("../../gen/java")
            java.directories.add("../../gen/kotlin")
            kotlin.directories.clear()
            kotlin.directories.add("../kotlin-codec/src/main/kotlin")
            kotlin.directories.add("../../gen/kotlin")
            jniLibs.directories.clear()
            jniLibs.directories.add(jniLibsDir.get().path)
            assets.directories.add(nativeAssetsDir.get().path)
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
    onlyIf { !configuredNativeAssetsDir.isPresent }
    inputs.dir(jniLibsDir)
    outputs.file(nativeAssetsDir.map { it.resolve(requiredAndroidNativeManifest) })
    doLast {
        val root = jniLibsDir.get()
        val nativeFiles = requiredAndroidJniLibs.map { relativePath ->
            val file = root.resolve(relativePath)
            if (!file.isFile) {
                throw GradleException("missing ReallyMe codec Android jniLib for manifest: $relativePath")
            }
            file
        }
        val commitSha = providers.environmentVariable("GITHUB_SHA").orNull
            ?: providers.exec {
                workingDir = layout.projectDirectory.dir("../..").asFile
                commandLine("git", "rev-parse", "HEAD")
            }.standardOutput.asText.get().trim()
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
        val aarFile = aarFiles.single()
        val missing = requiredAndroidJniLibs.filter { relativePath ->
            !zipTree(aarFile).matching {
                include("jni/$relativePath")
            }.files.any()
        }
        if (missing.isNotEmpty()) {
            throw GradleException(
                "release AAR is missing JNI entries: ${missing.joinToString(", ")}"
            )
        }
        val hasNativeManifest = zipTree(aarFile).matching {
            include("assets/$requiredAndroidNativeManifest")
        }.files.any()
        if (!hasNativeManifest) {
            throw GradleException(
                "release AAR is missing native manifest asset: $requiredAndroidNativeManifest"
            )
        }
    }
}

tasks.withType<PublishToMavenLocal>().configureEach {
    dependsOn("verifyReleaseAarContainsJniLibs")
}

tasks.withType<PublishToMavenRepository>().configureEach {
    dependsOn("verifyReleaseAarContainsJniLibs")
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
        maven {
            name = "localRelease"
            url = layout.buildDirectory.dir("repos/releases").get().asFile.toURI()
        }
        if (remoteMavenRepositoryUrl.isPresent) {
            maven {
                name = "remoteRelease"
                url = uri(remoteMavenRepositoryUrl.get())
                credentials {
                    username = remoteMavenUsername.orNull
                    password = remoteMavenPassword.orNull
                }
            }
        }
    }
}

signing {
    if (signingKey.isPresent) {
        useInMemoryPgpKeys(signingKey.get(), signingPassword.orNull)
        sign(publishing.publications["release"])
    }
}
