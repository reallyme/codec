// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#include <stddef.h>
#include <stdint.h>
#include <string.h>

extern uint32_t rm_codec_abi_version(void);
extern uint32_t rm_codec_package_version_major(void);
extern uint32_t rm_codec_package_version_minor(void);
extern uint32_t rm_codec_package_version_patch(void);
extern size_t rm_codec_max_ffi_output_bytes(void);
extern int32_t rm_codec_process(uint32_t operation, const uint8_t *first, size_t first_len,
                                const uint8_t *second, size_t second_len,
                                const uint8_t *third, size_t third_len,
                                uint8_t *output, size_t output_len, size_t *len_out);

enum {
    CODEC_ABI_VERSION = 6,
    CODEC_BASE64_ENCODE = 1,
    CODEC_OK = 0,
    CODEC_INVALID_ARGUMENT = -1,
    CODEC_BUFFER_TOO_SMALL = -5,
};

int main(void) {
    static const uint8_t input[] = {1, 2, 3};
    static const uint8_t expected[] = {'A', 'Q', 'I', 'D'};
    uint8_t output[4] = {0};
    size_t produced = 0;

    if (rm_codec_abi_version() != CODEC_ABI_VERSION ||
        rm_codec_package_version_major() != 0 ||
        rm_codec_package_version_minor() != 3 ||
        rm_codec_package_version_patch() != 0 ||
        rm_codec_max_ffi_output_bytes() < sizeof(output)) {
        return 1;
    }
    if (rm_codec_process(CODEC_BASE64_ENCODE, input, sizeof(input), NULL, 0, NULL, 0,
                         output, 2, &produced) != CODEC_BUFFER_TOO_SMALL ||
        produced != sizeof(expected)) {
        return 2;
    }
    if (rm_codec_process(CODEC_BASE64_ENCODE, input, sizeof(input), NULL, 0, NULL, 0,
                         output, sizeof(output), &produced) != CODEC_OK ||
        produced != sizeof(expected) || memcmp(output, expected, sizeof(expected)) != 0) {
        return 3;
    }
    if (rm_codec_process(CODEC_BASE64_ENCODE, NULL, sizeof(input), NULL, 0, NULL, 0,
                         output, sizeof(output), &produced) != CODEC_INVALID_ARGUMENT) {
        return 4;
    }
    return 0;
}
