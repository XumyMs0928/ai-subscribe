import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DesktopContractError, isIntelEvidenceDetailV1 } from "./desktop-api";
import { createTauriDesktopApi } from "./tauri-desktop-api";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const detail = () =>
    JSON.parse(
        readFileSync(
            resolve(
                process.cwd(),
                "../../contracts/fixtures/intel-detail/phase1-v1.json",
            ),
            "utf8",
        ),
    ) as unknown;

describe("intel evidence detail DesktopApi transport", () => {
    beforeEach(() => vi.mocked(invoke).mockReset());

    it("queries by stable item ID and opens by stable item plus provenance IDs", async () => {
        const response = detail();
        expect(isIntelEvidenceDetailV1(response)).toBe(true);
        const itemId = (response as { facts: { intel_item_id: string } }).facts
            .intel_item_id;
        const provenanceId = (
            response as { provenance: Array<{ provenance_id: string }> }
        ).provenance[0].provenance_id;
        vi.mocked(invoke)
            .mockResolvedValueOnce(response)
            .mockResolvedValueOnce({
                contract_version: 1,
                intel_item_id: itemId,
                provenance_id: provenanceId,
                status: "requested",
            });
        const api = createTauriDesktopApi();

        await expect(api.queryIntelEvidenceDetail(itemId)).resolves.toEqual(
            response,
        );
        await expect(
            api.openIntelOriginal(itemId, provenanceId),
        ).resolves.toMatchObject({ status: "requested" });
        expect(invoke).toHaveBeenNthCalledWith(
            1,
            "query_intel_evidence_detail_v1",
            {
                input: { contract_version: 1, intel_item_id: itemId },
            },
        );
        expect(invoke).toHaveBeenNthCalledWith(2, "open_intel_original_v1", {
            input: {
                contract_version: 1,
                intel_item_id: itemId,
                provenance_id: provenanceId,
            },
        });
    });

    it("rejects extra keys, contradictory identities and malformed local states", async () => {
        const response = detail() as Record<string, unknown>;
        expect(isIntelEvidenceDetailV1({ ...response, unexpected: true })).toBe(
            false,
        );
        expect(
            isIntelEvidenceDetailV1({
                ...response,
                rule_status: "unavailable",
            }),
        ).toBe(false);
        const facts = response.facts as Record<string, unknown>;
        const provenance = response.provenance as Array<
            Record<string, unknown>
        >;
        expect(
            isIntelEvidenceDetailV1({
                ...response,
                provenance: [
                    {
                        ...provenance[0],
                        publisher: "Different publisher",
                    },
                    ...provenance.slice(1),
                ],
            }),
        ).toBe(false);
        expect(
            isIntelEvidenceDetailV1({
                ...response,
                provenance: [
                    {
                        ...provenance[0],
                        display_url: "原文地址不可用",
                        availability_status: "unavailable",
                        can_open_original: false,
                    },
                    ...provenance.slice(1),
                ],
            }),
        ).toBe(true);
        expect(
            isIntelEvidenceDetailV1({
                ...response,
                provenance: [
                    {
                        ...provenance[0],
                        display_url: "原文地址不可用",
                        availability_status: "unavailable",
                        can_open_original: true,
                    },
                    ...provenance.slice(1),
                ],
            }),
        ).toBe(false);
        const itemId = (response.facts as { intel_item_id: string })
            .intel_item_id;
        vi.mocked(invoke).mockResolvedValue({
            ...response,
            facts: {
                ...facts,
                intel_item_id: `intel:${"f".repeat(64)}`,
            },
        });
        await expect(
            createTauriDesktopApi().queryIntelEvidenceDetail(itemId),
        ).rejects.toBeInstanceOf(DesktopContractError);
    });

    it("rejects malformed IDs, duplicate provenance and receipt identity drift", async () => {
        const api = createTauriDesktopApi();
        await expect(
            api.queryIntelEvidenceDetail("intel:short"),
        ).rejects.toBeInstanceOf(DesktopContractError);
        expect(invoke).not.toHaveBeenCalled();

        const response = detail() as {
            facts: { intel_item_id: string };
            provenance: Array<{ provenance_id: string }>;
        };
        expect(
            isIntelEvidenceDetailV1({
                ...response,
                provenance: [response.provenance[0], response.provenance[0]],
            }),
        ).toBe(false);
        const itemId = response.facts.intel_item_id;
        const provenanceId = response.provenance[0].provenance_id;
        vi.mocked(invoke).mockResolvedValue({
            contract_version: 1,
            intel_item_id: itemId,
            provenance_id: "prov:different",
            status: "requested",
        });
        await expect(
            api.openIntelOriginal(itemId, provenanceId),
        ).rejects.toBeInstanceOf(DesktopContractError);
    });
});
