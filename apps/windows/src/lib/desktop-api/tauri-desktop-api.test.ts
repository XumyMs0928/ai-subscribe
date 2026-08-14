import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { createTauriDesktopApi } from "./tauri-desktop-api";
import { DesktopCommandError, isDesktopApiError } from "./desktop-api";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

describe("Tauri DesktopApi transport", () => {
    beforeEach(() => vi.mocked(invoke).mockReset());

    it("uses the only approved release command and preserves contract fields", async () => {
        vi.mocked(invoke).mockResolvedValue({
            contract_version: 1,
            status: "ok",
            checked_at: null,
        });

        const result = await createTauriDesktopApi().health();

        expect(invoke).toHaveBeenCalledOnce();
        expect(invoke).toHaveBeenCalledWith("health_v1");
        expect(result).toEqual({
            contract_version: 1,
            status: "ok",
            checked_at: null,
        });
    });

    it("rejects a malformed response instead of inventing contract defaults", async () => {
        vi.mocked(invoke).mockResolvedValue({
            contract_version: 1,
            status: "ok",
        });

        await expect(createTauriDesktopApi().health()).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
    });

    it("validates and preserves every stable AppError field", () => {
        const wire = {
            contract_version: 1 as const,
            code: "internal.unexpected",
            category: "internal",
            message_key: "error.internal",
            retryability: "manual",
            source_id: null,
            task_id: null,
            details_allowlisted: "",
            correlation_id: "windows-command-1",
        };
        expect(isDesktopApiError(wire)).toBe(true);
        const observed = new DesktopCommandError(wire);
        expect(observed).toMatchObject(wire);
    });

    it("rejects non-UTC and malformed checked_at values", async () => {
        for (const checked_at of [
            "2026-08-14T00:00:00+00:00",
            "2026-02-30T00:00:00Z",
            "not-a-time",
        ]) {
            vi.mocked(invoke).mockResolvedValue({
                contract_version: 1,
                status: "ok",
                checked_at,
            });
            await expect(
                createTauriDesktopApi().health(),
            ).rejects.toMatchObject({
                code: "internal.desktop_contract_mismatch",
            });
        }
    });

    it("uses the exact demo commands and preserves an empty search result", async () => {
        const catalog = {
            contract_version: 1,
            dataset_id: "demo-v1",
            items: [],
        };
        vi.mocked(invoke)
            .mockResolvedValueOnce({
                ...catalog,
                items: [
                    {
                        id: "demo:rust-001",
                        data_origin: "demo",
                        publisher: "Rust Project",
                        title: "Rust update",
                        track: "tools",
                        summary: "Demo summary",
                        original_url: "https://www.rust-lang.org/",
                        published_at: "2026-06-20T10:00:00Z",
                        collected_at: "2026-06-20T10:30:00Z",
                    },
                ],
            })
            .mockResolvedValueOnce(catalog);
        const api = createTauriDesktopApi();

        await expect(api.demoBootstrap()).resolves.toMatchObject({
            dataset_id: "demo-v1",
        });
        await expect(api.demoSearch("量子", "research")).resolves.toEqual(
            catalog,
        );

        expect(invoke).toHaveBeenNthCalledWith(1, "demo_bootstrap_v1");
        expect(invoke).toHaveBeenNthCalledWith(2, "demo_search_v1", {
            query: "量子",
            track: "research",
        });
    });

    it("rejects an empty bootstrap catalog but accepts it for search", async () => {
        const empty = { contract_version: 1, dataset_id: "demo-v1", items: [] };
        vi.mocked(invoke).mockResolvedValue(empty);
        await expect(
            createTauriDesktopApi().demoBootstrap(),
        ).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
        await expect(
            createTauriDesktopApi().demoSearch("missing", null),
        ).resolves.toEqual(empty);
    });

    it("validates demo detail responses at the transport boundary", async () => {
        vi.mocked(invoke).mockResolvedValue({
            id: "real:forged",
            data_origin: "demo",
        });

        await expect(
            createTauriDesktopApi().demoDetail("demo:missing"),
        ).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
        expect(invoke).toHaveBeenCalledWith("demo_detail_v1", {
            id: "demo:missing",
        });
    });

    it("uses explicit paged list and filter commands", async () => {
        const page = {
            contract_version: 1,
            dataset_id: "demo-v1",
            items: [],
            next_cursor: "offset:2",
        };
        vi.mocked(invoke).mockResolvedValue(page);
        const api = createTauriDesktopApi();

        await expect(api.demoList(null, 2)).resolves.toEqual(page);
        await expect(api.demoFilter("tools", "offset:2", 2)).resolves.toEqual(
            page,
        );
        expect(invoke).toHaveBeenNthCalledWith(1, "demo_list_v1", {
            cursor: null,
            limit: 2,
        });
        expect(invoke).toHaveBeenNthCalledWith(2, "demo_filter_v1", {
            track: "tools",
            cursor: "offset:2",
            limit: 2,
        });
    });
});
