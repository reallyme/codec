# SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
#
# SPDX-License-Identifier: Apache-2.0

-keep class me.really.codec.ReallyMeCodecNative { *; }
-keep class me.really.codec.ReallyMeCodecException { *; }
-keep class me.really.codec.ReallyMeCodecException$* { *; }

# Protobuf Lite reflects on generated backing-field names from the encoded
# schema metadata at runtime. Consumer R8 must not rename these generated
# message classes or members, or optimized apps fail while constructing or
# parsing codec operation envelopes.
-keep class me.really.codec.v1.** { *; }
