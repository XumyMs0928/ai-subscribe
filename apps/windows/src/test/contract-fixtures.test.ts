import { describe, expect, it } from "vitest";

import { isHealthStatusV1 } from "../lib/desktop-api/desktop-api";
import { loadGoldenFixture } from "./load-golden-fixture";

interface GoldenFixture {
    readonly scenario: string;
    readonly input: unknown;
    readonly expected: Record<string, unknown>;
    readonly forbidden_fields: readonly string[];
}

function fixture(name: string): GoldenFixture {
    return loadGoldenFixture(name) as GoldenFixture;
}

describe("Rust-authoritative golden fixtures", () => {
    it("accepts the shared health fixture including explicit null", () => {
        const health = fixture("health_success_v1.json");

        expect(isHealthStatusV1(health.expected)).toBe(true);
        expect(health.expected).toHaveProperty("checked_at", null);
    });

    it("preserves shared validation, internal, effect, and secret semantics", () => {
        const validation = fixture("validation_failure_v1.json");
        const internal = fixture("internal_error_v1.json");
        const effect = fixture("effect_report_v1.json");
        const secret = fixture("secret_lease_v1.json");

        expect(validation.expected).toMatchObject({
            contract_version: 1,
            code: "validation.effect_id",
            category: "validation",
        });
        expect(internal.expected).toMatchObject({
            contract_version: 1,
            code: "internal.unexpected",
            category: "internal",
        });
        expect(effect.expected).toEqual({
            first: "applied",
            repeat: "already_applied",
            conflict: "conflict.effect_already_reported",
        });
        expect(secret.expected).toMatchObject({
            second_use: "conflict.secret_lease_consumed",
            observable_canary_hits: 0,
        });
    });
});
