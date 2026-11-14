// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { test } from "node:test";
import { findManualProtoPresenceFields } from "./source-policy.mjs";

test("schema policy rejects manual presence flags, including inline declarations", () => {
  assert.deepEqual(findManualProtoPresenceFields(`
    message Request { bytes kid = 1; bool has_kid = 2; }
    message Limits {
      optional bool limit_present = 1;
      bool algorithm_is_set = 2;
    }
  `), ["has_kid", "limit_present", "algorithm_is_set"]);
});

test("schema policy ignores comments and string options", () => {
  assert.deepEqual(findManualProtoPresenceFields(`
    // bool has_old_field = 1;
    /* bool payload_present = 2; */
    option description = "bool has_example = 3;";
    message Request { optional bytes payload = 1; }
  `), []);
});

test("schema policy permits actual bool values and native presence", () => {
  assert.deepEqual(findManualProtoPresenceFields(`
    message Value {
      optional bool preferred = 1;
      bool variable_length = 2;
      bool valid = 3;
      optional string algorithm = 4;
      oneof payload { bytes attached = 5; bool detached = 6; }
    }
  `), []);
});
