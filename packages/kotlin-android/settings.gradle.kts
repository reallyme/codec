// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

pluginManagement {
    repositories {
        google()
        gradlePluginPortal()
        mavenCentral()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
        providers.gradleProperty("reallyme.codec.androidPublicationRepository").orNull?.let { path ->
            exclusiveContent {
                forRepository {
                    maven { url = uri(path) }
                }
                filter { includeModule("me.really", "codec-android") }
            }
        }
    }
}

rootProject.name = "reallyme-codec-android"

include(":consumer-r8-runtime")
