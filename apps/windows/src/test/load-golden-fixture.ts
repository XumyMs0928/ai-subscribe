import { readFileSync } from "node:fs";
import { join } from "node:path";

export function loadGoldenFixture(name: string): unknown {
    const fixturePath = join(
        process.cwd(),
        "..",
        "..",
        "contracts",
        "fixtures",
        "golden",
        name,
    );
    return JSON.parse(readFileSync(fixturePath, "utf8")) as unknown;
}
