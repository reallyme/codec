#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 ReallyMe LLC
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Derive published runtime dependency records from Gradle's full locks.

Gradle also locks Android lint, compiler, and test tooling. Those artifacts are
not shipped in either SDK, so the release vulnerability gate scans the runtime
classpaths derived here while the full locks remain pinned for builds.
"""

import argparse
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
LANES = (
    ("packages/kotlin/gradle.lockfile", "packages/kotlin/runtime/gradle.lockfile", "runtimeClasspath"),
    (
        "packages/kotlin-android/gradle.lockfile",
        "packages/kotlin-android/runtime/gradle.lockfile",
        "releaseRuntimeClasspath",
    ),
)
REQUIRED_ARTIFACTS = frozenset(
    {
        "com.google.protobuf:protobuf-javalite",
        "com.google.protobuf:protobuf-kotlin-lite",
        "org.jetbrains.kotlin:kotlin-stdlib",
    }
)


def runtime_records(source: str, configuration: str) -> str:
    """Keep only coordinates Gradle resolved for the published runtime."""
    selected = set()
    for line in source.splitlines():
        if not line or line.startswith("#"):
            continue
        coordinate, separator, configurations = line.partition("=")
        if not separator or not configurations:
            raise ValueError("malformed Gradle dependency lock record")
        if coordinate == "empty":
            continue
        if len(coordinate.split(":")) != 3:
            raise ValueError("malformed Gradle dependency coordinate")
        names = configurations.split(",")
        if not all(names) or len(names) != len(set(names)):
            raise ValueError("malformed Gradle dependency configuration list")
        if configuration in names:
            selected.add(coordinate)

    artifacts = {coordinate.rsplit(":", 1)[0] for coordinate in selected}
    if not REQUIRED_ARTIFACTS.issubset(artifacts):
        raise ValueError("published Gradle runtime dependencies are incomplete")

    header = (
        "# Derived from the Gradle lockfile for the published runtime classpath.\n"
        "# Regenerate with: python3 scripts/gradle_runtime_lockfiles.py --write\n"
    )
    return header + "".join(
        f"{coordinate}={configuration}\n" for coordinate in sorted(selected)
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--write", action="store_true")
    args = parser.parse_args()

    for source, target, configuration in LANES:
        generated = runtime_records((ROOT / source).read_text(), configuration)
        destination = ROOT / target
        if args.write:
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(generated)
        elif not destination.exists() or destination.read_text() != generated:
            raise SystemExit(f"{target} is stale; regenerate the runtime lockfile")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
