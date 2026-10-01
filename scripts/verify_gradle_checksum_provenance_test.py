# SPDX-FileCopyrightText: 2026 ReallyMe LLC
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Regression tests for independently reviewing Gradle verification hashes."""

import hashlib
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts import verify_gradle_checksum_provenance as provenance


class ProvenanceTests(unittest.TestCase):
    def test_metadata_rejects_missing_digest_and_unsafe_coordinate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            metadata = root / "metadata.xml"
            template = (
                '<verification-metadata xmlns="https://schema.gradle.org/dependency-verification">'
                '<components><component group="{group}" name="item" version="1.0">'
                '<artifact name="item-1.0.jar">{digest}</artifact>'
                '</component></components></verification-metadata>'
            )
            with patch.object(provenance, "REPOSITORY_ROOT", root), patch.object(
                provenance, "FILES", ("metadata.xml",)
            ):
                metadata.write_text(template.format(group="safe.group", digest=""))
                with self.assertRaisesRegex(ValueError, "lacks a SHA-256"):
                    provenance.entries()
                metadata.write_text(
                    template.format(
                        group="../unsafe",
                        digest='<sha256 value="' + ("0" * 64) + '"/>',
                    )
                )
                with self.assertRaisesRegex(ValueError, "invalid Gradle verification coordinate"):
                    provenance.entries()

    def test_sha256_sidecar_matches_pinned_digest(self):
        payload = b"publisher artifact"
        digest = hashlib.sha256(payload).hexdigest()
        response = subprocess.CompletedProcess(args=(), returncode=0, stdout=digest, stderr="")
        item = (("example.org", "artifact", "1.0", "artifact-1.0.jar"), digest)
        with patch.object(provenance.subprocess, "run", return_value=response):
            status, _, _ = provenance.verify(item)
        self.assertEqual(status, "match-sha256")

    def test_preferred_repository_mismatch_cannot_be_masked_by_mirror(self):
        item = (("example.org", "artifact", "1.0", "artifact-1.0.jar"), "0" * 64)
        responses = [
            subprocess.CompletedProcess(args=(), returncode=0, stdout="1" * 64, stderr=""),
            subprocess.CompletedProcess(args=(), returncode=0, stdout="0" * 64, stderr=""),
        ]
        with patch.object(provenance.subprocess, "run", side_effect=responses) as fetch:
            status, source, _ = provenance.verify(item)
        self.assertEqual(status, "mismatch")
        self.assertEqual(source, provenance.REPOSITORIES[0])
        self.assertEqual(fetch.call_count, 1)

    def test_artifact_fallback_hashes_publisher_bytes(self):
        payload = b"published bytes"
        digest = hashlib.sha256(payload).hexdigest()
        item = (("example.org", "artifact", "1.0", "artifact-1.0.jar"), digest)
        def published_artifact(args, **_):
            if "-o" in args:
                Path(args[args.index("-o") + 1]).write_bytes(payload)
                return subprocess.CompletedProcess(args, 0, "", "")
            return subprocess.CompletedProcess(args, 22, "", "not published")

        with patch.object(provenance.subprocess, "run", side_effect=published_artifact):
            status, _, _ = provenance.verify(item)
        self.assertEqual(status, "match-artifact-sha256")

    def test_first_repository_artifact_precedes_later_mirror_sidecar(self):
        payload = b"plugin portal marker"
        digest = hashlib.sha256(payload).hexdigest()
        item = (("org.example", "org.example.gradle.plugin", "1.0", "org.example.gradle.plugin-1.0.pom"), digest)

        def publisher_response(args, **_):
            if "-o" in args:
                Path(args[args.index("-o") + 1]).write_bytes(payload)
                return subprocess.CompletedProcess(args, 0, "", "")
            return subprocess.CompletedProcess(args, 22, "", "no sidecar")

        with patch.object(provenance.subprocess, "run", side_effect=publisher_response) as fetch:
            status, source, _ = provenance.verify(item)
        self.assertEqual(status, "match-artifact-sha256")
        self.assertEqual(source, provenance.REPOSITORIES[2])
        self.assertEqual(fetch.call_count, 2)

    def test_android_plugin_marker_uses_google_repository_first(self):
        digest = "0" * 64
        item = (("com.android.library", "com.android.library.gradle.plugin", "9.4.1", "com.android.library.gradle.plugin-9.4.1.pom"), digest)
        response = subprocess.CompletedProcess(args=(), returncode=0, stdout=digest, stderr="")
        with patch.object(provenance.subprocess, "run", return_value=response) as fetch:
            status, source, _ = provenance.verify(item)
        self.assertEqual(status, "match-sha256")
        self.assertEqual(source, provenance.REPOSITORIES[1])
        self.assertTrue(fetch.call_args.args[0][-1].startswith(provenance.REPOSITORIES[1]))

    def test_mismatched_artifact_hash_is_rejected(self):
        item = (("example.org", "artifact", "1.0", "artifact-1.0.jar"), "0" * 64)

        def changed_artifact(args, **_):
            if "-o" in args:
                Path(args[args.index("-o") + 1]).write_bytes(b"different bytes")
                return subprocess.CompletedProcess(args, 0, "", "")
            return subprocess.CompletedProcess(args, 22, "", "not published")

        with patch.object(provenance.subprocess, "run", side_effect=changed_artifact):
            status, _, _ = provenance.verify(item)
        self.assertEqual(status, "mismatch")

    def test_unavailable_artifact_is_not_accepted(self):
        item = (("example.org", "artifact", "1.0", "artifact-1.0.jar"), "0" * 64)
        unavailable = subprocess.CompletedProcess(args=(), returncode=22, stdout="", stderr="missing")
        with patch.object(provenance.subprocess, "run", return_value=unavailable):
            status, _, _ = provenance.verify(item)
        self.assertEqual(status, "unavailable")

    def test_timeout_resumes_partial_artifact_before_hashing(self):
        payload = b"publisher artifact after resume"
        digest = hashlib.sha256(payload).hexdigest()
        item = (("example.org", "artifact", "1.0", "artifact-1.0.jar"), digest)
        attempts = 0

        def interrupted_download(args, **_):
            nonlocal attempts
            if "-o" not in args:
                return subprocess.CompletedProcess(args, 22, "", "no sidecar")
            attempts += 1
            destination = Path(args[args.index("-o") + 1])
            self.assertIn("-C", args)
            if attempts == 1:
                destination.write_bytes(payload[:10])
                return subprocess.CompletedProcess(args, 28, "", "timeout")
            destination.write_bytes(payload)
            return subprocess.CompletedProcess(args, 0, "", "")

        with patch.object(provenance.subprocess, "run", side_effect=interrupted_download):
            status, _, _ = provenance.verify(item)
        self.assertEqual(status, "match-artifact-sha256")
        self.assertEqual(attempts, 2)


if __name__ == "__main__":
    unittest.main()
