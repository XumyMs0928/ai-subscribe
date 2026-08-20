import { describe, expect, it } from "vitest";

import {
    isConfigurationValidationResultV1,
    isHealthStatusV1,
    isSetupProgressV1,
    isSourceViewV1,
} from "../lib/desktop-api/desktop-api";
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

    it("accepts the Rust-owned setup progress fixture with explicit nulls", () => {
        const setup = loadGoldenFixture("setup_progress_v1.json");
        expect(isSetupProgressV1(setup)).toBe(true);
        expect(setup).toHaveProperty("next_step_id", "tracks");
        expect(setup).toHaveProperty("saved_config.refresh_cadence", "manual");
    });

    it("accepts the Rust-produced configuration validation wire result", () => {
        const validation = fixture("configuration_validation_v1.json");

        expect(isConfigurationValidationResultV1(validation.expected)).toBe(
            true,
        );
        expect(validation.expected).toMatchObject({
            blocking_errors: [],
            narrowing_risks: [],
            validator_version: "attention-configuration-v1",
            validation_receipt: null,
        });
        expect(validation.forbidden_fields).toContain("force_save");
    });

    it("accepts the shared redacted RSS source view", () => {
        const source = fixture("source_view_v1.json");
        expect(isSourceViewV1(source.expected)).toBe(true);
        expect(source.expected.display_url).toBe(
            "https://example.com/feed.xml",
        );
        expect(source.forbidden_fields).toContain("canonical_url");
    });
});
