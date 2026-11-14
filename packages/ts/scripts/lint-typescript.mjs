// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { API } from "typescript/unstable/sync";
import { SyntaxKind } from "typescript/unstable/ast";
import { isAbsolute, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = fileURLToPath(new URL("../", import.meta.url));
const sourceRoot = resolve(packageRoot, "src");
const config = resolve(packageRoot, "tsconfig.json");

// Use the pinned compiler's syntax tree so comments, literals, and identifiers
// that merely spell a reserved word do not hide or trigger unsafe expressions.
function failure(node) {
  switch (node.kind) {
    case SyntaxKind.AnyKeyword:
      return "explicit any";
    case SyntaxKind.NonNullExpression:
      return "non-null assertion";
    case SyntaxKind.TypeAssertionExpression:
      return "type assertion";
    case SyntaxKind.AsExpression:
      return node.type.getText() === "const" ? undefined : "type assertion";
    case SyntaxKind.NamespaceImport:
    case SyntaxKind.NamespaceExport:
      return "wildcard surface";
    case SyntaxKind.ExportDeclaration:
      return node.exportClause === undefined ? "wildcard export" : undefined;
    default:
      return undefined;
  }
}

const compiler = new API({ cwd: packageRoot });
try {
  const snapshot = compiler.updateSnapshot({ openProjects: [config] });
  const project = snapshot.getProject(config);
  if (project === undefined) {
    console.error("TypeScript source lint could not load the pinned project");
    process.exitCode = 1;
  } else {
    for (const file of project.rootFiles) {
      const path = relative(sourceRoot, file);
      if (isAbsolute(path) || path === ".." || path.startsWith(`..${sep}`)) continue;
      if (path.split(sep).includes("generated")) continue;
      const source = project.program.getSourceFile(file);
      if (source === undefined) {
        console.error("TypeScript source lint could not load a project file");
        process.exitCode = 1;
        continue;
      }
      const inspect = (node) => {
        const violation = failure(node);
        if (violation !== undefined) {
          console.error(`${path}:${node.getStart()}: ${violation}`);
          process.exitCode = 1;
        }
        node.forEachChild(inspect);
      };
      inspect(source);
    }
  }
  snapshot.dispose();
} finally {
  compiler.close();
}
