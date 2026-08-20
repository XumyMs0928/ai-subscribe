import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
    DesktopContractError,
    isIntelFeedPageV1,
    isQueryIntelFeedInputV1,
} from "./desktop-api";
import { createTauriDesktopApi } from "./tauri-desktop-api";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const query = {
    contract_version: 1,
    stream: "high_value",
    filters: {
        track_ids: ["ai_agents"],
        source_ids: ["source:111111111111111111111111"],
        time_window: "last_7d",
        importance: ["high"],
    },
    sort: "score_desc",
    cursor: null,
    limit: 30,
} as const;

const page = () => ({
    contract_version: 1,
    stream: "high_value" as const,
    filters: query.filters,
    sort: "score_desc" as const,
    rule_version: "rss-intelligence-value-v1",
    configuration_revision: 1,
    configuration_hash: "1".repeat(64),
    as_of_ms: 1_777_000_000_000,
    items: [
        {
            contract_version: 1,
            intel_item_id: `intel:${"2".repeat(64)}`,
            source_id: "source:111111111111111111111111",
            source_kind: "rss_atom" as const,
            publisher: "publisher.example",
            title: "AI agent security release",
            source_excerpt: "A bounded source excerpt.",
            excerpt_truncated: false,
            published_at: "2026-08-20T08:00:00Z",
            collected_at: "2026-08-20T08:05:00Z",
            importance: "high" as const,
            score: 95,
            matched_track_ids: ["ai_agents"],
            stream_disposition: "high_value" as const,
            ai_status: "unavailable" as const,
        },
    ],
    next_cursor: null,
});

describe("intel feed DesktopApi transport", () => {
    beforeEach(() => vi.mocked(invoke).mockReset());

    it("sends the narrow command and accepts the exact page contract", async () => {
        vi.mocked(invoke).mockResolvedValue(page());

        await expect(
            createTauriDesktopApi().queryIntelFeed(query),
        ).resolves.toEqual(page());
        expect(invoke).toHaveBeenCalledWith("query_intel_feed_v1", {
            input: query,
        });
    });

    it("fails closed on contradictory feed projection metadata", async () => {
        vi.mocked(invoke).mockResolvedValue({
            ...page(),
            stream: "ordinary_candidate",
        });

        await expect(
            createTauriDesktopApi().queryIntelFeed(query),
        ).rejects.toBeInstanceOf(DesktopContractError);
    });

    it("rejects malformed identifiers, extra keys and non-canonical filters", () => {
        expect(isQueryIntelFeedInputV1(query)).toBe(true);
        expect(
            isQueryIntelFeedInputV1({
                ...query,
                filters: { ...query.filters, track_ids: ["z", "a"] },
            }),
        ).toBe(false);
        expect(
            isIntelFeedPageV1({
                ...page(),
                items: [{ ...page().items[0], intel_item_id: "intel:short" }],
            }),
        ).toBe(false);
        expect(isIntelFeedPageV1({ ...page(), unexpected: true })).toBe(false);
    });

    it("accepts core-compatible opaque track ids and semantic filter key order", async () => {
        const extended = {
            ...query,
            filters: {
                ...query.filters,
                track_ids: ["Agent.Tools:release-candidate"],
            },
        } as const;
        expect(isQueryIntelFeedInputV1(extended)).toBe(true);
        const response = page();
        vi.mocked(invoke).mockResolvedValue({
            ...response,
            filters: {
                importance: extended.filters.importance,
                time_window: extended.filters.time_window,
                source_ids: extended.filters.source_ids,
                track_ids: extended.filters.track_ids,
            },
            items: [
                {
                    ...response.items[0],
                    matched_track_ids: ["Agent.Tools:release-candidate"],
                },
            ],
        });

        await expect(
            createTauriDesktopApi().queryIntelFeed(extended),
        ).resolves.toMatchObject({ filters: extended.filters });
    });

    it("rejects zero as-of, tie misordering, duplicates and pages above the request limit", async () => {
        expect(isIntelFeedPageV1({ ...page(), as_of_ms: 0 })).toBe(false);
        const first = page().items[0];
        const reverseTie = {
            ...first,
            intel_item_id: `intel:${"1".repeat(64)}`,
        };
        expect(
            isIntelFeedPageV1({ ...page(), items: [first, reverseTie] }),
        ).toBe(false);
        expect(isIntelFeedPageV1({ ...page(), items: [first, first] })).toBe(
            false,
        );

        vi.mocked(invoke).mockResolvedValue({
            ...page(),
            items: Array.from({ length: 31 }, (_, index) => ({
                ...first,
                intel_item_id: `intel:${index.toString(16).padStart(64, "0")}`,
                score: 100 - index,
            })),
        });
        await expect(
            createTauriDesktopApi().queryIntelFeed(query),
        ).rejects.toBeInstanceOf(DesktopContractError);
    });
});
