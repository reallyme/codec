#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Check package member paths without extracting files or printing contents."""

import argparse
import gzip
import io
import os
from pathlib import Path, PurePosixPath
import stat
import struct
import subprocess
import sys
import tarfile
import zipfile

MAX_FILE_BYTES = 256 * 1024 * 1024
MAX_DIRECTORY_BYTES = 8 * 1024 * 1024
MAX_EXPANDED_BYTES = 128 * 1024 * 1024
MAX_ENTRIES = 20_000
MAX_DEPTH = 4
MAX_PATH_BYTES = 4096
ARCHIVE_SUFFIXES = (".zip", ".jar", ".aar", ".crate", ".tgz", ".tar.gz", ".tar")
ROOT = Path(__file__).resolve().parent.parent


class PrivacyError(Exception):
    """Fixed reason codes keep package contents out of CI diagnostics."""


def member_path(name):
    if not isinstance(name, str) or len(name.encode("utf8")) > MAX_PATH_BYTES:
        raise PrivacyError("INVALID_MEMBER_PATH")
    parts = name.split("/")
    if (not name or name.startswith("/") or "\\" in name or "\0" in name
            or ":" in parts[0] or ".." in parts):
        raise PrivacyError("INVALID_MEMBER_PATH")
    return str(PurePosixPath(name))


def check_paths(paths, npm, android=False):
    if android:
        # This standard AAR metadata uses "build" as a Java package segment,
        # not as a workspace output directory. Admit only that metadata path;
        # sibling files and private configuration still follow the ignore rules.
        metadata = "META-INF/com/android/build/gradle/aar-metadata.properties"
        containers = {str(parent) for parent in PurePosixPath(metadata).parents}
        paths = [path for path in paths if path != metadata and path not in containers]
    encoded = ("\0".join(paths) + "\0").encode("utf8")
    if len(encoded) > MAX_DIRECTORY_BYTES:
        raise PrivacyError("MEMBER_LIMIT")
    result = subprocess.run(
        ["node", str(ROOT / "scripts/check_ignored_files.mjs"),
         "--npm-paths" if npm else "--paths"],
        cwd=ROOT, input=encoded, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        timeout=60, check=False,
    )
    if result.returncode != 0:
        raise PrivacyError("FORBIDDEN_PACKAGE_MEMBER")


class BoundedReader:
    def __init__(self, source, state):
        self.source = source
        self.state = state

    def read(self, size):
        if size < 0 or size > MAX_DIRECTORY_BYTES:
            raise PrivacyError("ARCHIVE_LIMIT")
        data = self.source.read(size)
        if len(data) > MAX_EXPANDED_BYTES - self.state["streamed"]:
            raise PrivacyError("ARCHIVE_LIMIT")
        self.state["streamed"] += len(data)
        return data


def zip_members(source):
    # Validate the central-directory budget before ZipFile allocates its index.
    # ZIP64 packages are outside this release policy and fail closed.
    source.seek(0, os.SEEK_END)
    length = source.tell()
    if length > MAX_FILE_BYTES:
        raise PrivacyError("ARCHIVE_LIMIT")
    source.seek(max(0, length - 65_557))
    tail = source.read()
    position = tail.rfind(b"PK\x05\x06")
    if position < 0 or len(tail) - position < 22:
        raise PrivacyError("INVALID_ARCHIVE")
    record = struct.unpack_from("<4s4H2LH", tail, position)
    if (record[1] != 0 or record[2] != 0 or record[3] != record[4]
            or record[4] > MAX_ENTRIES or record[5] > MAX_DIRECTORY_BYTES
            or record[6] == 0xFFFFFFFF):
        raise PrivacyError("ARCHIVE_LIMIT")
    source.seek(0)
    return zipfile.ZipFile(source)


def admit(state, count=0, expanded=0):
    if (count > MAX_ENTRIES - state["entries"]
            or expanded > MAX_EXPANDED_BYTES - state["expanded"]):
        raise PrivacyError("ARCHIVE_LIMIT")
    state["entries"] += count
    state["expanded"] += expanded


def inspect_archive(source, name, npm, state, depth=0):
    if depth > MAX_DEPTH:
        raise PrivacyError("ARCHIVE_LIMIT")
    if name.lower().endswith((".zip", ".jar", ".aar")):
        with zip_members(source) as archive:
            entries = archive.infolist()
            if len(entries) > MAX_ENTRIES:
                raise PrivacyError("ARCHIVE_LIMIT")
            admit(state, count=len(entries))
            paths = []
            for entry in entries:
                paths.append(member_path(entry.filename))
                if stat.S_ISLNK(entry.external_attr >> 16):
                    raise PrivacyError("LINK_MEMBER")
            check_paths(paths, npm, android=name.lower().endswith(".aar"))
            for entry in entries:
                if entry.filename.lower().endswith(ARCHIVE_SUFFIXES) and not entry.is_dir():
                    admit(state, expanded=entry.file_size)
                    data = archive.read(entry)
                    inspect_archive(io.BytesIO(data), entry.filename, npm, state, depth + 1)
        return

    source.seek(0)
    compressed = name.lower().endswith((".gz", ".tgz", ".crate"))
    stream = gzip.GzipFile(fileobj=source) if compressed else source
    try:
        with tarfile.open(fileobj=BoundedReader(stream, state), mode="r|") as archive:
            paths = []
            nested = []
            for entry in archive:
                admit(state, count=1)
                if entry.size > MAX_EXPANDED_BYTES:
                    raise PrivacyError("ARCHIVE_LIMIT")
                paths.append(member_path(entry.name))
                if not entry.isdir() and not entry.isfile():
                    raise PrivacyError("LINK_MEMBER")
                if entry.isfile() and entry.name.lower().endswith(ARCHIVE_SUFFIXES):
                    admit(state, expanded=entry.size)
                    member = archive.extractfile(entry)
                    if member is None:
                        raise PrivacyError("INVALID_ARCHIVE")
                    with member:
                        nested.append((entry.name, member.read()))
            check_paths(paths, npm)
            for nested_name, contents in nested:
                inspect_archive(io.BytesIO(contents), nested_name, npm, state, depth + 1)
    finally:
        if compressed:
            stream.close()


def inspect_paths(paths, npm):
    state = {"entries": 0, "expanded": 0, "streamed": 0}
    for supplied in paths:
        root = Path(supplied)
        if root.is_symlink() or not root.exists():
            raise PrivacyError("INVALID_PACKAGE_PATH")
        candidates = root.rglob("*") if root.is_dir() else [root]
        files = []
        names = []
        for path in candidates:
            admit(state, count=1)
            if path.is_symlink():
                raise PrivacyError("PACKAGE_LIMIT")
            relative = path.relative_to(root) if root.is_dir() else Path(path.name)
            names.append(member_path(relative.as_posix()))
            files.append(path)
        check_paths(names, npm)
        for path in files:
            if path.is_file() and path.name.lower().endswith(ARCHIVE_SUFFIXES):
                if path.stat().st_size > MAX_FILE_BYTES:
                    raise PrivacyError("ARCHIVE_LIMIT")
                with path.open("rb") as source:
                    inspect_archive(source, path.name, npm, state)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--npm", action="store_true")
    parser.add_argument("paths", nargs="+")
    arguments = parser.parse_args()
    try:
        inspect_paths(arguments.paths, arguments.npm)
    except PrivacyError as error:
        print("release privacy check failed: " + str(error), file=sys.stderr)
        return 1
    except (OSError, ValueError, EOFError, RuntimeError, NotImplementedError,
            tarfile.TarError, zipfile.BadZipFile,
            subprocess.SubprocessError, UnicodeError):
        print("release privacy check failed: INVALID_PACKAGE", file=sys.stderr)
        return 1
    print("release payload privacy check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
