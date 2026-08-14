import { readFileSync, readdirSync } from "node:fs";
import { extname, join, relative } from "node:path";
import { describe, expect, it } from "vitest";

const sourceRoot = join(process.cwd(), "src");

function sourceFiles(directory: string): string[] {
    return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
        const path = join(directory, entry.name);
        if (entry.isDirectory()) return sourceFiles(path);
        return [".ts", ".tsx"].includes(extname(path)) &&
            !path.endsWith(".test.ts") &&
            !path.endsWith(".test.tsx")
            ? [path]
            : [];
    });
}

describe("production source boundaries", () => {
    it("keeps raw Tauri invoke inside the DesktopApi transport", () => {
        const violations = sourceFiles(sourceRoot)
            .filter((path) =>
                readFileSync(path, "utf8").includes("@tauri-apps/api/core"),
            )
            .map((path) => relative(sourceRoot, path).replaceAll("\\", "/"))
            .filter((path) => path !== "lib/desktop-api/tauri-desktop-api.ts");

        expect(violations).toEqual([]);
    });

    it("contains no database, generic command, or plaintext-secret surface", () => {
        const rustSqlite = ["rus", "qlite"].join("");
        const forbiddenSurface = new RegExp(
            `\\b(?:invokeAny|execute|${rustSqlite}|sqlite|secret[_-]?value)\\b`,
            "i",
        );
        const sqlStatement = /\\b(?:SELECT|INSERT|UPDATE|DELETE)\\s+\\b/;
        const violations = sourceFiles(sourceRoot)
            .filter((path) => {
                const source = readFileSync(path, "utf8");
                return (
                    forbiddenSurface.test(source) || sqlStatement.test(source)
                );
            })
            .map((path) => relative(sourceRoot, path).replaceAll("\\", "/"));

        expect(violations).toEqual([]);
    });
});
