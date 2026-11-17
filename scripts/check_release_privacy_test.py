# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

import io
from pathlib import Path
import re
import stat
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import check_release_privacy as privacy


class ReleasePrivacyTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="codec-package-privacy-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def zip(self, name, members):
        path = self.root / name
        with zipfile.ZipFile(path, "w") as archive:
            for member, contents in members.items():
                archive.writestr(member, contents)
        return path

    def tar(self, name, members):
        path = self.root / name
        with tarfile.open(path, "w:gz") as archive:
            for member, contents in members.items():
                entry = tarfile.TarInfo(member)
                entry.size = len(contents)
                archive.addfile(entry, io.BytesIO(contents))
        return path

    def test_native_payload_is_allowed(self):
        path = self.zip("native.zip", {"Framework/Info.plist": b"public", "Framework/library.a": b"binary"})
        privacy.inspect_paths([path], False)

    def test_local_instruction_and_configuration_paths_are_rejected(self):
        rules = (privacy.ROOT / ".gitignore").read_text()
        section = rules.split("# Local environment\n", 1)[1].split("\n\n", 1)[0]
        directories = re.findall(r"^\.[a-z][a-z0-9-]*/$", section, re.MULTILINE)
        for name in ["AGENTS.md", ".env", *[directory + "file" for directory in directories]]:
            with self.subTest(name=name):
                path = self.zip("private.zip", {"package/" + name: b"local"})
                with self.assertRaisesRegex(privacy.PrivacyError, "FORBIDDEN_PACKAGE_MEMBER"):
                    privacy.inspect_paths([path], False)

    def test_npm_allows_distribution_code_but_preserves_private_rules(self):
        path = self.tar("npm.tgz", {"package/dist/index.js": b"public"})
        privacy.inspect_paths([path], True)
        with self.assertRaisesRegex(privacy.PrivacyError, "FORBIDDEN_PACKAGE_MEMBER"):
            privacy.inspect_paths([path], False)
        private = self.tar("private.tgz", {"package/dist/AGENTS.md": b"local"})
        with self.assertRaisesRegex(privacy.PrivacyError, "FORBIDDEN_PACKAGE_MEMBER"):
            privacy.inspect_paths([private], True)

    def test_nested_archives_are_checked(self):
        inner = self.zip("inner.jar", {"AGENTS.md": b"local"})
        outer = self.zip("outer.zip", {"library.jar": inner.read_bytes()})
        with self.assertRaisesRegex(privacy.PrivacyError, "FORBIDDEN_PACKAGE_MEMBER"):
            privacy.inspect_paths([outer], False)

    def test_standard_android_metadata_is_allowed_without_allowing_private_siblings(self):
        metadata = "META-INF/com/android/build/gradle/aar-metadata.properties"
        path = self.zip("android.aar", {metadata: b"aarFormatVersion=1.0\n"})
        privacy.inspect_paths([path], False)
        private = self.zip("private.aar", {metadata: b"public", "META-INF/com/android/build/AGENTS.md": b"local"})
        with self.assertRaisesRegex(privacy.PrivacyError, "FORBIDDEN_PACKAGE_MEMBER"):
            privacy.inspect_paths([private], False)

    def test_tar_nesting_is_checked(self):
        inner = self.tar("inner.tgz", {"AGENTS.md": b"local"})
        outer = self.tar("outer.tgz", {"library.tgz": inner.read_bytes()})
        with self.assertRaisesRegex(privacy.PrivacyError, "FORBIDDEN_PACKAGE_MEMBER"):
            privacy.inspect_paths([outer], False)

    def test_traversal_and_platform_escape_paths_are_rejected(self):
        for name in ["../file", "/file", "C:/file", "folder\\file", "a/../../file"]:
            with self.subTest(name=name):
                path = self.zip("escape.zip", {name: b"data"})
                with self.assertRaisesRegex(privacy.PrivacyError, "INVALID_MEMBER_PATH"):
                    privacy.inspect_paths([path], False)

    def test_links_are_rejected_without_following_them(self):
        path = self.root / "link.zip"
        entry = zipfile.ZipInfo("link")
        entry.create_system = 3
        entry.external_attr = (stat.S_IFLNK | 0o777) << 16
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr(entry, "../private")
        with self.assertRaisesRegex(privacy.PrivacyError, "LINK_MEMBER"):
            privacy.inspect_paths([path], False)
        linked = self.root / "linked"
        linked.symlink_to(path)
        with self.assertRaisesRegex(privacy.PrivacyError, "INVALID_PACKAGE_PATH"):
            privacy.inspect_paths([linked], False)

    def test_metadata_size_entry_and_recursion_budgets_fail_closed(self):
        path = self.zip("large.zip", {"file": b"public"})
        with patch.object(privacy, "MAX_DIRECTORY_BYTES", 1), patch.object(privacy, "check_paths"):
            with self.assertRaisesRegex(privacy.PrivacyError, "ARCHIVE_LIMIT"):
                privacy.inspect_paths([path], False)
        with patch.object(privacy, "MAX_FILE_BYTES", 1):
            with self.assertRaisesRegex(privacy.PrivacyError, "ARCHIVE_LIMIT"):
                privacy.inspect_paths([path], False)
        with patch.object(privacy, "MAX_ENTRIES", 1):
            with self.assertRaisesRegex(privacy.PrivacyError, "ARCHIVE_LIMIT"):
                privacy.inspect_paths([path], False)
        outer = self.zip("outer.zip", {"inner.zip": path.read_bytes()})
        with patch.object(privacy, "MAX_DEPTH", 0):
            with self.assertRaisesRegex(privacy.PrivacyError, "ARCHIVE_LIMIT"):
                privacy.inspect_paths([outer], False)

    def test_expanded_data_budget_applies_across_nested_members(self):
        inner = self.zip("inner.zip", {"file": b"public"})
        outer = self.zip("outer.zip", {"first.zip": inner.read_bytes(), "second.zip": inner.read_bytes()})
        with patch.object(privacy, "MAX_EXPANDED_BYTES", inner.stat().st_size):
            with self.assertRaisesRegex(privacy.PrivacyError, "ARCHIVE_LIMIT"):
                privacy.inspect_paths([outer], False)

    def test_missing_and_malformed_archives_fail_closed(self):
        with self.assertRaisesRegex(privacy.PrivacyError, "INVALID_PACKAGE_PATH"):
            privacy.inspect_paths([self.root / "missing"], False)
        invalid = self.root / "invalid.zip"
        invalid.write_bytes(b"not a zip")
        with self.assertRaisesRegex(privacy.PrivacyError, "INVALID_ARCHIVE"):
            privacy.inspect_paths([invalid], False)


if __name__ == "__main__":
    unittest.main()
