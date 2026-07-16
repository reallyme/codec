// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import org.gradle.api.publish.maven.tasks.PublishToMavenLocal
import org.gradle.api.publish.maven.tasks.PublishToMavenRepository
import org.gradle.external.javadoc.StandardJavadocDocletOptions

plugins {
    kotlin("jvm") version "2.4.0"
    `java-library`
    `maven-publish`
    signing
}

group = "me.really"
version = "0.1.21"

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
val requireRemoteMavenPublishing = providers.gradleProperty("reallyme.maven.requireRemote")
    .map { it == "true" }
    .orElse(false)

fun nonBlank(value: String?): String? = value?.trim()?.takeIf { it.isNotEmpty() }

val remoteMavenRepositoryUrlValue = nonBlank(remoteMavenRepositoryUrl.orNull)
val remoteMavenUsernameValue = nonBlank(remoteMavenUsername.orNull)
val remoteMavenPasswordValue = nonBlank(remoteMavenPassword.orNull)
val signingKeyValue = nonBlank(signingKey.orNull)
val signingPasswordValue = nonBlank(signingPassword.orNull)
val configuredNativeResourcesDir = providers.gradleProperty("reallyme.codec.nativeResourcesDir")
val nativeResourcesDir = configuredNativeResourcesDir
    .map { file(it) }
    .orElse(layout.buildDirectory.dir("generated/native-resources").map { it.asFile })
val requireFullNativeResources = providers.gradleProperty("reallyme.codec.requireFullNativeResources")
    .map { it == "true" }
    .orElse(false)
val requiredNativeResources = listOf(
    "me/really/codec/native/linux-x86_64/libreallyme_codec_ffi.so",
    "me/really/codec/native/linux-aarch64/libreallyme_codec_ffi.so",
    "me/really/codec/native/macos-x86_64/libreallyme_codec_ffi.dylib",
    "me/really/codec/native/macos-aarch64/libreallyme_codec_ffi.dylib",
    "me/really/codec/native/windows-x86_64/reallyme_codec_ffi.dll",
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
    commandLine("cargo", "build", "-p", "reallyme-codec-ffi", "--release")
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

dependencies {
    api("com.google.protobuf:protobuf-javalite:4.35.1")
    api("com.google.protobuf:protobuf-kotlin-lite:4.35.1")
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
    dependsOn(stageHostNativeResource)
}

tasks.named("sourcesJar") {
    dependsOn(stageHostNativeResource)
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
    }
}

val verifyHostBundledNativeResource = tasks.register("verifyHostBundledNativeResource") {
    group = "verification"
    description = "Verifies that local JVM artifacts include the host Rust JNI library."
    dependsOn(stageHostNativeResource)
    inputs.dir(nativeResourcesDir)
    doLast {
        val root = nativeResourcesDir.get()
        if (!root.resolve(requiredHostNativeResource).isFile) {
            throw GradleException(
                "missing ReallyMe codec host native resource: $requiredHostNativeResource"
            )
        }
    }
}

tasks.withType<PublishToMavenLocal>().configureEach {
    dependsOn(verifyHostBundledNativeResource)
    if (requireFullNativeResources.get()) {
        dependsOn(verifyBundledNativeResources)
    }
}

tasks.withType<PublishToMavenRepository>().configureEach {
    dependsOn(verifyBundledNativeResources)
}

val verifyRemoteMavenPublishingConfigured = tasks.register("verifyRemoteMavenPublishingConfigured") {
    group = "verification"
    description = "Verifies that remote Maven publishing credentials are configured."
    onlyIf { requireRemoteMavenPublishing.get() }
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
        maven {
            name = "localRelease"
            url = layout.buildDirectory.dir("repos/releases").get().asFile.toURI()
        }
        if (remoteMavenRepositoryUrlValue != null) {
            maven {
                name = "remoteRelease"
                url = uri(remoteMavenRepositoryUrlValue)
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
