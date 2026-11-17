// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#if canImport(Darwin)
    import Darwin
#endif

/// Snapshot identity and mutation state in one stat call. ctime detects writes
/// even when an owner restores the previous file length and modification time.
struct LoadedFileIdentity: Equatable, Sendable {
    let device: Int64
    let inode: UInt64
    let size: Int64
    let modifiedSeconds: Int64
    let modifiedNanoseconds: Int64
    let changedSeconds: Int64
    let changedNanoseconds: Int64

    init?(path: String) {
        #if canImport(Darwin)
            var status = stat()
            guard stat(path, &status) == 0, (status.st_mode & S_IFMT) == S_IFREG,
                let device = Int64(exactly: status.st_dev),
                let inode = UInt64(exactly: status.st_ino),
                let size = Int64(exactly: status.st_size),
                let modifiedSeconds = Int64(exactly: status.st_mtimespec.tv_sec),
                let modifiedNanoseconds = Int64(exactly: status.st_mtimespec.tv_nsec),
                let changedSeconds = Int64(exactly: status.st_ctimespec.tv_sec),
                let changedNanoseconds = Int64(exactly: status.st_ctimespec.tv_nsec)
            else {
                return nil
            }
            self.device = device
            self.inode = inode
            self.size = size
            self.modifiedSeconds = modifiedSeconds
            self.modifiedNanoseconds = modifiedNanoseconds
            self.changedSeconds = changedSeconds
            self.changedNanoseconds = changedNanoseconds
        #else
            _ = path
            return nil
        #endif
    }
}
