import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type {
    DemoCatalogV1,
    DesktopApi,
} from "../../lib/desktop-api/desktop-api";
import { createAppQueryClient } from "../../lib/query-client";
import { DesktopApiProvider } from "../../app/providers/desktop-api-provider";
import { DemoIntelligence } from "./demo-intelligence";

const demo: DemoCatalogV1 = {
    contract_version: 1,
    dataset_id: "demo-v1",
    items: [
        {
            id: "demo:rust-001",
            data_origin: "demo",
            publisher: "Rust Project",
            title: "Rust 工具链更新",
            track: "开发工具",
            summary: "固定演示摘要",
            original_url: "https://www.rust-lang.org/",
            published_at: "2026-06-20T10:00:00Z",
            collected_at: "2026-06-20T10:30:00Z",
        },
    ],
};

function api(overrides: Partial<DesktopApi> = {}): DesktopApi {
    return {
        health: vi.fn().mockResolvedValue({
            contract_version: 1,
            status: "ok",
            checked_at: null,
        }),
        demoBootstrap: vi.fn().mockResolvedValue(demo),
        demoSearch: vi.fn().mockResolvedValue(demo),
        demoList: vi.fn().mockResolvedValue({ ...demo, next_cursor: null }),
        demoFilter: vi.fn().mockResolvedValue({ ...demo, next_cursor: null }),
        demoDetail: vi.fn().mockResolvedValue(demo.items[0]),
        ...overrides,
    };
}

function renderShell(desktopApi: DesktopApi) {
    return render(
        <QueryClientProvider client={createAppQueryClient()}>
            <DesktopApiProvider api={desktopApi}>
                <DemoIntelligence />
            </DesktopApiProvider>
        </QueryClientProvider>,
    );
}

describe("Windows demo intelligence shell", () => {
    it("loads the shared demo catalog and marks list and detail without color-only meaning", async () => {
        renderShell(api());
        expect(screen.getByRole("status")).toHaveTextContent(
            "正在加载演示数据",
        );
        expect(
            await screen.findByRole("button", { name: /Rust 工具链更新/ }),
        ).toBeVisible();
        expect(await screen.findByText("固定演示摘要")).toBeVisible();
        expect(
            screen.getByRole("region", { name: "演示情报详情" }),
        ).toHaveTextContent("演示数据");
        expect(screen.getAllByText("演示数据").length).toBeGreaterThanOrEqual(
            2,
        );
    });

    it("searches through DesktopApi and keeps demo labels in results", async () => {
        const demoSearch = vi.fn().mockResolvedValue(demo);
        const desktopApi = api({ demoSearch });
        renderShell(desktopApi);
        const search = await screen.findByRole("textbox", { name: "搜索" });
        await userEvent.type(search, "Rust");
        await userEvent.click(screen.getByRole("button", { name: "搜索" }));
        expect(
            (await screen.findAllByText("Rust 工具链更新")).length,
        ).toBeGreaterThanOrEqual(1);
        expect(demoSearch).toHaveBeenCalledWith("Rust", null);
        expect(screen.getAllByText("演示数据").length).toBeGreaterThanOrEqual(
            2,
        );
    });

    it("shows a persistent, recoverable error without private transport details", async () => {
        const demoBootstrap = vi
            .fn()
            .mockRejectedValueOnce(new Error("private detail"))
            .mockResolvedValueOnce(demo);
        renderShell(api({ demoBootstrap }));
        const alert = await screen.findByRole("alert");
        expect(alert).not.toHaveTextContent("private detail");
        await userEvent.click(screen.getByRole("button", { name: "重试" }));
        expect(
            await screen.findByRole("heading", { name: "Rust 工具链更新" }),
        ).toBeVisible();
    });

    it("does not request network, AI credentials, or notification permission", async () => {
        const fetchSpy = vi.spyOn(globalThis, "fetch");
        renderShell(api());
        expect(await screen.findByText("Rust 工具链更新")).toBeVisible();
        expect(fetchSpy).not.toHaveBeenCalled();
        fetchSpy.mockRestore();
    });

    it("shows recoverable health and detail errors", async () => {
        const health = vi
            .fn()
            .mockRejectedValueOnce(new Error("private health"))
            .mockResolvedValueOnce({
                contract_version: 1,
                status: "ok",
                checked_at: null,
            });
        const demoDetail = vi
            .fn()
            .mockRejectedValueOnce(new Error("private detail"))
            .mockResolvedValueOnce(demo.items[0]);
        renderShell(api({ health, demoDetail }));

        expect(
            await screen.findByRole("button", { name: "重新连接" }),
        ).toBeVisible();
        await userEvent.click(screen.getByRole("button", { name: "重新连接" }));
        expect(await screen.findByText(/共享核心 healthy/)).toBeVisible();

        expect(
            await screen.findByRole("button", { name: "重试详情" }),
        ).toBeVisible();
        await userEvent.click(screen.getByRole("button", { name: "重试详情" }));
        expect(await screen.findByText("固定演示摘要")).toBeVisible();
    });
});
