# SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
#
# SPDX-License-Identifier: Apache-2.0

-keep class me.really.codec.ReallyMeCodecNative { *; }
-keep class me.really.codec.ReallyMeCodecException { *; }
-keep class me.really.codec.ReallyMeCodecException$* { *; }

# Rust resolves these names and members directly through JNI. Renaming either
# type, the enum fields, or the result constructor would make every protobuf
# result operation fail at runtime in an optimized consumer application.
-keep class me.really.codec.ReallyMeCodecProtoStatus { *; }
-keep class me.really.codec.ReallyMeCodecProtoResult { *; }
