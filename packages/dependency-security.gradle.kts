// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Install the same rules before plugins resolve and before project classpaths
// are created. Otherwise vulnerable plugin dependencies escape project locks.
val dependencyVersions = mapOf(
    "org.jetbrains.kotlin:kotlin-gradle-plugin" to "2.4.21",
    "org.bitbucket.b_c:jose4j" to "0.9.7",
    "org.jdom:jdom2" to "2.0.6.1",
    "com.google.auto.value:auto-value-annotations" to "1.11.1",
    "com.google.crypto.tink:tink" to "1.24.0",
    "com.google.dagger:dagger" to "2.60.1",
    "com.google.guava:failureaccess" to "1.0.3",
    "com.google.j2objc:j2objc-annotations" to "3.1",
    "com.google.protobuf:protobuf-java" to "4.36.2",
    "com.google.protobuf:protobuf-java-util" to "4.36.2",
    "com.squareup:javawriter" to "2.5.1",
    "net.sf.jopt-simple:jopt-simple" to "5.0.4",
    "org.checkerframework:checker-qual" to "4.4.0",
    "org.jetbrains:annotations" to "26.1.0",
    "org.jetbrains.kotlin:kotlin-stdlib-jdk7" to "2.4.21",
    "org.jetbrains.kotlin:kotlin-stdlib-jdk8" to "2.4.21",
    "org.jetbrains.kotlinx:kotlinx-coroutines-bom" to "1.11.0",
    "org.jetbrains.kotlinx:kotlinx-coroutines-core" to "1.11.0",
    "org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm" to "1.11.0",
    "org.ow2.asm:asm-util" to "9.11.1",
    "ch.qos.logback:logback-classic" to "1.6.8",
    "ch.qos.logback:logback-core" to "1.6.8",
    "com.google.guava:guava" to "33.8.0-jre",
    "org.apache.commons:commons-lang3" to "3.22.0",
    "org.apache.httpcomponents:httpclient" to "4.5.14",
    "org.apache.groovy:groovy" to "4.0.33",
    "org.apache.groovy:groovy-bom" to "4.0.33",
    "org.glassfish.jaxb:jaxb-runtime" to "2.3.9",
    "org.glassfish.jaxb:txw2" to "2.3.9",
    "com.sun.istack:istack-commons-runtime" to "3.0.12",
    "com.sun.xml.fastinfoset:FastInfoset" to "1.2.18",
    "jakarta.activation:jakarta.activation-api" to "1.2.2",
    "jakarta.xml.bind:jakarta.xml.bind-api" to "2.3.3",
    "org.jvnet.staxex:stax-ex" to "1.8.3",
    "com.github.ajalt.clikt:clikt" to "5.2.1",
    "com.github.ajalt.clikt:clikt-jvm" to "5.2.1",
    "com.google.code.gson:gson" to "2.15.0",
    "com.google.errorprone:error_prone_annotations" to "2.53.0",
    "com.google.jimfs:jimfs" to "1.3.2",
    "commons-codec:commons-codec" to "1.23.0",
    "commons-io:commons-io" to "2.22.0",
    "commons-logging:commons-logging" to "1.4.0",
    "net.java.dev.jna:jna" to "5.19.1",
    "net.java.dev.jna:jna-platform" to "5.19.1",
    "org.apache.commons:commons-compress" to "1.28.0",
    "org.apache.httpcomponents:httpmime" to "4.5.14",
    "org.ow2.asm:asm" to "9.11.1",
    "org.ow2.asm:asm-analysis" to "9.11.1",
    "org.ow2.asm:asm-commons" to "9.11.1",
    "org.ow2.asm:asm-tree" to "9.11.1",
    "org.slf4j:slf4j-api" to "2.0.20",
    "org.jspecify:jspecify" to "1.0.1",
)

gradle.beforeProject {
    val useReviewedDependencies = Action<org.gradle.api.artifacts.Configuration> {
        resolutionStrategy.eachDependency {
            val selected = when {
                requested.group == "org.bouncycastle" && requested.name.contains("jdk18on") ->
                    "1.87"
                else -> dependencyVersions["${requested.group}:${requested.name}"]
            }
            if (selected != null) {
                useVersion(selected)
                because("Use reviewed compatible releases across SDK and build-tool classpaths")
            }
        }
    }
    buildscript.configurations.configureEach(useReviewedDependencies)
    buildscript.configurations.named("classpath") {
        resolutionStrategy.activateDependencyLocking()
    }
    configurations.configureEach(useReviewedDependencies)
}
