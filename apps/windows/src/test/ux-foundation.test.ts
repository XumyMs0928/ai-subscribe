import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

describe("Windows UX foundation", () => {
    const styles = readFileSync(
        join(process.cwd(), "src/styles/globals.css"),
        "utf8",
    );

    it("defines the DESIGN typography, spacing, border, focus, and motion tokens", () => {
        for (const token of [
            "--type-display-token",
            "--type-h2-token",
            "--type-body-token",
            "--space-1-token",
            "--space-8-token",
            "--border-default-token",
            "--focus-width-token",
            "--motion-fast-token",
        ]) {
            expect(styles).toContain(token);
        }
    });

    it("supports dark mode, Windows forced colors, reduced motion, and zoom-safe sizing", () => {
        expect(styles).toContain("prefers-color-scheme: dark");
        expect(styles).toContain("forced-colors: active");
        expect(styles).toContain("prefers-reduced-motion: reduce");
        expect(styles).toMatch(/min-width:\s*20rem/);
        expect(styles).not.toMatch(/min-width:\s*\d+px/);
    });
});
