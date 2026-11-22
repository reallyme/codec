#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Corroborate Gradle verification hashes against publisher repository bytes."""

from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import hashlib
import re
import subprocess
import tempfile
from pathlib import Path
import xml.etree.ElementTree as ET


FILES = (
    "packages/kotlin/gradle/verification-metadata.xml",
    "packages/kotlin-android/gradle/verification-metadata.xml",
)
REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
NAMESPACE = {"g": "https://schema.gradle.org/dependency-verification"}
REPOSITORIES = (
    "https://repo.maven.apache.org/maven2/",
    "https://dl.google.com/dl/android/maven2/",
    "https://plugins.gradle.org/m2/",
)
ARTIFACT_DOWNLOAD_TIMEOUT_SECONDS = 600
ARTIFACT_DOWNLOAD_ATTEMPTS = 3
MAX_CONCURRENT_DOWNLOADS = 4
CURL_RETRY_ATTEMPTS = 2
CURL_RETRY_BUDGET_SECONDS = 30
CURL_HTTP_STATUS = re.compile(r"CODEC_HTTP_STATUS:([0-9]{3})")
CURL_TRANSPORT_OPTIONS = (
    "--user-agent", "reallyme-codec-dependency-verification/1 (+https://github.com/reallyme/codec)",
    "--retry", str(CURL_RETRY_ATTEMPTS),
    "--retry-max-time", str(CURL_RETRY_BUDGET_SECONDS),
    "--write-out", "%{stderr}CODEC_HTTP_STATUS:%{http_code}\\n",
)
COORDINATE_PART = re.compile(r"[A-Za-z0-9_][A-Za-z0-9._+-]*\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")

# Gradle records the variant's file name; its published module metadata points
# to a different Maven filename. Verify the publisher bytes at that URL.
PUBLISHED_ARTIFACT_NAMES = {
    ("com.github.ajalt.clikt", "clikt-jvm", "5.0.2", "clikt-mordant-jvm-sources.jar"):
        "clikt-jvm-5.0.2-sources.jar",
    ("com.github.ajalt.clikt", "clikt-jvm", "5.0.3", "clikt-mordant-jvm-sources.jar"):
        "clikt-jvm-5.0.3-sources.jar",
    ("com.github.ajalt.clikt", "clikt-jvm", "5.2.1", "clikt-mordant-jvm-5.2.1-sources.jar"):
        "clikt-jvm-5.2.1-sources.jar",
}


def validated_coordinate_part(value):
    if value is None or ".." in value or COORDINATE_PART.fullmatch(value) is None:
        raise ValueError("invalid Gradle verification coordinate")
    return value


def entries():
    """Read all declared artifact hashes, rejecting missing or conflicting entries."""
    result = {}
    for filename in FILES:
        root = ET.parse(REPOSITORY_ROOT / filename).getroot()
        for component in root.findall("g:components/g:component", NAMESPACE):
            group, name, version = (
                validated_coordinate_part(component.get(key))
                for key in ("group", "name", "version")
            )
            for artifact in component.findall("g:artifact", NAMESPACE):
                digest = artifact.find("g:sha256", NAMESPACE)
                if digest is None:
                    raise ValueError("artifact lacks a SHA-256 verification hash")
                key = (group, name, version, validated_coordinate_part(artifact.get("name")))
                value = digest.get("value")
                if value is None or SHA256.fullmatch(value.lower()) is None:
                    raise ValueError("invalid Gradle verification SHA-256 hash")
                expected = value.lower()
                if key in result and result[key] != expected:
                    raise ValueError(f"conflicting metadata: {key}")
                result[key] = expected
    return result


def verify(item):
    """Return the publisher evidence for one locally pinned artifact hash."""
    (group, name, version, artifact), expected = item
    published_name = PUBLISHED_ARTIFACT_NAMES.get((group, name, version, artifact), artifact)
    path = f"{group.replace('.', '/')}/{name}/{version}/{published_name}.sha256"
    if group.startswith(("androidx.", "com.android.", "com.google.android.")):
        preferred = (REPOSITORIES[1], REPOSITORIES[0], REPOSITORIES[2])
    elif name.endswith(".gradle.plugin"):
        preferred = (REPOSITORIES[2], REPOSITORIES[0], REPOSITORIES[1])
    else:
        preferred = REPOSITORIES
    failures = []

    def transport_failure(base, kind, completed):
        # Record only fixed repository names and numeric transport status;
        # backend error text is unnecessary for diagnosing publisher outages.
        match = CURL_HTTP_STATUS.search(completed.stderr)
        status = int(match.group(1)) if match else 0
        return (base, kind, completed.returncode, status)

    # Resolve one repository completely before considering the next. Plugin
    # marker POMs can differ byte-for-byte between the plugin portal and Maven
    # Central, so a later mirror's sidecar must not override the first source's
    # matching artifact when that source omits SHA-256 sidecars.
    with tempfile.TemporaryDirectory(prefix="codec-gradle-check-") as directory:
        destination = Path(directory) / "artifact"
        for base in preferred:
            destination.unlink(missing_ok=True)
            sidecar = subprocess.run(
                (
                    "curl", "-fLsS", "--proto", "=https", "--proto-redir", "=https",
                    *CURL_TRANSPORT_OPTIONS,
                    "--max-time", "8", "--max-filesize", "1024", base + path,
                ),
                capture_output=True,
                text=True,
                check=False,
            )
            if sidecar.returncode == 0:
                words = sidecar.stdout.strip().split()
                published = words[0].lower() if words else ""
                if published == expected:
                    return ("match-sha256", base, (group, name, version, artifact))
                return ("mismatch", base, (group, name, version, artifact, expected, published))
            failures.append(transport_failure(base, "sidecar", sidecar))

            # A fresh publisher download corroborates exact bytes when a
            # SHA-256 sidecar is unavailable. SHA-1 is never sufficient.
            for _ in range(ARTIFACT_DOWNLOAD_ATTEMPTS):
                # A range request is useful only for bytes retained from a
                # previous timeout, never for a fresh or empty output file.
                resume = ("-C", "-") if destination.exists() and destination.stat().st_size > 0 else ()
                completed = subprocess.run(
                    (
                        "curl", "-fLsS", "--proto", "=https", "--proto-redir", "=https",
                        *CURL_TRANSPORT_OPTIONS,
                        "--max-time", str(ARTIFACT_DOWNLOAD_TIMEOUT_SECONDS),
                        "--max-filesize", "134217728",
                        *resume, "-o", str(destination), base + path[:-7],
                    ),
                    capture_output=True,
                    text=True,
                    check=False,
                )
                if completed.returncode != 28:
                    break
                # Curl preserves the authenticated HTTPS bytes already
                # received. Resume only timeouts, then hash the complete file
                # so a changed publisher response cannot pass verification.
            if completed.returncode != 0:
                failures.append(transport_failure(base, "artifact", completed))
                continue
            with destination.open("rb") as stream:
                published_sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
            if published_sha256 == expected:
                return ("match-artifact-sha256", base, (group, name, version, artifact))
            return ("mismatch", base, (group, name, version, artifact, expected, published_sha256))

    return ("unavailable", "", (group, name, version, artifact, tuple(failures)))


if __name__ == "__main__":
    all_entries = entries()
    # Keep concurrent publisher traffic bounded, including during retries.
    with ThreadPoolExecutor(max_workers=MAX_CONCURRENT_DOWNLOADS) as executor:
        results = list(executor.map(verify, all_entries.items()))
    counts = Counter(result[0] for result in results)
    sources = Counter(result[1] for result in results if result[0].startswith("match"))
    print(f"unique entries: {len(all_entries)}; results: {dict(counts)}")
    print(f"matched repositories: {dict(sources)}")
    for status in ("mismatch", "unavailable"):
        values = [result[2] for result in results if result[0] == status]
        print(status, values[:30])
    if counts["mismatch"] or counts["unavailable"]:
        raise SystemExit(1)
