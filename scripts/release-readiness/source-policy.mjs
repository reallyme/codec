// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

/**
 * Replace source comments with whitespace while preserving strings and lines.
 *
 * Architecture checks must inspect executable source rather than accepting a
 * required call that survives only in a comment. Preserving offsets and line
 * endings keeps diagnostics stable and allows the same scanner to cover the
 * repository's Rust, TypeScript, Swift, and Kotlin source blocks.
 */
const hasClosingSingleQuoteOnLine = (source, start) => {
  for (let index = start + 1; index < source.length; index += 1) {
    const character = source[index];
    if (character === "\n") {
      return false;
    }
    if (character === "\\") {
      index += 1;
    } else if (character === "'") {
      return true;
    }
  }
  return false;
};

const isRustLifetimeStart = (source, index) => {
  const previous = source[index - 1] ?? "";
  const next = source[index + 1] ?? "";
  return /[<&+:,]/u.test(previous) && /[A-Za-z_]/u.test(next);
};

const isRegexLiteralStart = (source, index) => {
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const character = source[cursor];
    if (/\s/u.test(character)) {
      continue;
    }
    return /[=(:,[!&|?{};]/u.test(character);
  }
  return true;
};

export const stripSourceComments = (source, options = {}) => {
  if (typeof source !== "string") {
    throw new TypeError("source must be a string");
  }
  const nestedBlockComments = options.nestedBlockComments ?? false;
  if (typeof nestedBlockComments !== "boolean") {
    throw new TypeError("nestedBlockComments must be a boolean");
  }

  let output = "";
  let state = "normal";
  let blockDepth = 0;
  let regexCharacterClass = false;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    const next = source[index + 1];

    if (state === "normal") {
      if (character === "/" && next === "/") {
        output += "  ";
        index += 1;
        state = "line-comment";
      } else if (character === "/" && next === "*") {
        output += "  ";
        index += 1;
        blockDepth = 1;
        state = "block-comment";
      } else if (character === "/" && isRegexLiteralStart(source, index)) {
        output += character;
        regexCharacterClass = false;
        state = "regex-literal";
      } else if (
        character === '"' ||
        character === "`" ||
        (character === "'" &&
          !isRustLifetimeStart(source, index) &&
          hasClosingSingleQuoteOnLine(source, index))
      ) {
        output += character;
        state = character;
      } else {
        output += character;
      }
      continue;
    }

    if (state === "line-comment") {
      if (character === "\n") {
        output += "\n";
        state = "normal";
      } else {
        output += " ";
      }
      continue;
    }

    if (state === "block-comment") {
      if (nestedBlockComments && character === "/" && next === "*") {
        output += "  ";
        index += 1;
        blockDepth += 1;
      } else if (character === "*" && next === "/") {
        output += "  ";
        index += 1;
        blockDepth -= 1;
        if (blockDepth === 0) {
          state = "normal";
        }
      } else {
        output += character === "\n" ? "\n" : " ";
      }
      continue;
    }

    if (state === "regex-literal") {
      output += character;
      if (character === "\\" && next !== undefined) {
        output += next;
        index += 1;
      } else if (character === "[") {
        regexCharacterClass = true;
      } else if (character === "]") {
        regexCharacterClass = false;
      } else if (character === "/" && !regexCharacterClass) {
        state = "normal";
      }
      continue;
    }

    output += character;
    if (character === "\\" && next !== undefined) {
      output += next;
      index += 1;
    } else if (character === state) {
      state = "normal";
    }
  }
  return output;
};

/** Replace quoted string contents as well as comments with whitespace. */
export const stripSourceStringsAndComments = (source) => {
  const uncommented = stripSourceComments(source);
  let output = "";
  let quote;
  let regexCharacterClass = false;
  for (let index = 0; index < uncommented.length; index += 1) {
    const character = uncommented[index];
    const next = uncommented[index + 1];
    if (quote === undefined) {
      if (character === "/" && isRegexLiteralStart(uncommented, index)) {
        output += " ";
        quote = "regex-literal";
        regexCharacterClass = false;
        continue;
      }
      if (
        character === '"' ||
        character === "`" ||
        (character === "'" &&
          !isRustLifetimeStart(uncommented, index) &&
          hasClosingSingleQuoteOnLine(uncommented, index))
      ) {
        output += " ";
        quote = character;
      } else {
        output += character;
      }
      continue;
    }
    if (quote === "regex-literal") {
      if (character === "\\" && next !== undefined) {
        output += next === "\n" ? " \n" : "  ";
        index += 1;
      } else if (character === "[") {
        output += " ";
        regexCharacterClass = true;
      } else if (character === "]") {
        output += " ";
        regexCharacterClass = false;
      } else if (character === "/" && !regexCharacterClass) {
        output += " ";
        quote = undefined;
      } else {
        output += character === "\n" ? "\n" : " ";
      }
      continue;
    }
    if (character === "\\" && next !== undefined) {
      output += next === "\n" ? " \n" : "  ";
      index += 1;
    } else if (character === quote) {
      output += " ";
      quote = undefined;
    } else {
      output += character === "\n" ? "\n" : " ";
    }
  }
  return output;
};

export const sourceBlockFromNeedle = ({
  source,
  startNeedle,
  nextNeedle,
}) => {
  for (const [name, value] of Object.entries({ source, startNeedle, nextNeedle })) {
    if (typeof value !== "string" || value.length === 0) {
      throw new TypeError(`${name} must be a non-empty string`);
    }
  }
  const executableSource = stripSourceComments(source);
  const start = executableSource.indexOf(startNeedle);
  if (start === -1) {
    return undefined;
  }
  const next = executableSource.indexOf(
    nextNeedle,
    start + startNeedle.length,
  );
  return executableSource.slice(
    start,
    next === -1 ? executableSource.length : next,
  );
};
