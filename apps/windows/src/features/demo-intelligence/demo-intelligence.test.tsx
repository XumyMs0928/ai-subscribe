import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type {
    AiStatusV1,
    DemoCatalogV1,
    DemoEvidenceDetailV1,
    DemoItemV1,
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
            importance: "high",
            ai_status: "generated",
            published_at: "2026-06-20T10:00:00Z",
            collected_at: "2026-06-20T10:30:00Z",
        },
    ],
};
const secondDemoItem: DemoItemV1 = {
    ...demo.items[0],
    id: "demo:rust-002",
    title: "Rust 第二条情报",
    original_url: "https://www.rust-lang.org/learn",
};

function detailFor(
    item: DemoItemV1 = demo.items[0],
    aiStatus: AiStatusV1 = "generated",
): DemoEvidenceDetailV1 {
    return {
        ...item,
        ai_status: aiStatus,
        contract_version: 1,
        dataset_id: "demo-v1",
        what_happened: "发生了什么",
        why_it_matters: "为什么重要",
        possible_impact: "可能影响",
        facts: ["原始事实"],
        rule_reasons: ["规则判断"],
        ai_content: "演示 AI 生成",
        ai_confidence_percent: 88,
        provenance: {
            source_kind: "official_release",
            publisher: item.publisher,
            author: null,
            original_title: item.title,
            original_url: item.original_url,
            published_at: item.published_at,
            collected_at: item.collected_at,
            first_discovered_at: item.collected_at,
            last_updated_at: item.collected_at,
            availability_status: "available",
            deterministic_association_basis: "demo_fixture_id",
        },
    };
}

function api(overrides: Partial<DesktopApi> = {}): DesktopApi {
    const detail = detailFor();
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
        demoDetail: vi.fn().mockResolvedValue(detail),
        getSyncResult: vi.fn(),
        queryIntelFeed: vi.fn(),
        queryIntelEvidenceDetail: vi.fn(),
        openIntelOriginal: vi.fn(),
        setupProgress: vi.fn(),
        saveSetupStep: vi.fn(),
        configuration: vi.fn(),
        validateConfiguration: vi.fn(),
        saveConfiguration: vi.fn(),
        saveSource: vi.fn(),
        querySources: vi.fn(),
        startSync: vi.fn(),
        task: vi.fn(),
        syncHealth: vi.fn(),
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
        expect(
            screen.getByRole("heading", { name: "发生了什么" }),
        ).toBeVisible();
        expect(
            screen.getByRole("heading", { name: "为什么重要" }),
        ).toBeVisible();
        expect(screen.getByRole("heading", { name: "原始事实" })).toBeVisible();
        expect(screen.getByRole("heading", { name: "规则判断" })).toBeVisible();
        expect(
            screen.getByRole("heading", { name: "演示 AI 生成" }),
        ).toBeVisible();
        expect(screen.getByRole("heading", { name: "来源溯源" })).toBeVisible();
    });

    it("keeps focus and selection distinct across keyboard list-detail navigation", async () => {
        renderShell(api());
        const item = await screen.findByRole("button", {
            name: /Rust 工具链更新/,
        });
        item.focus();
        expect(item).toHaveFocus();
        expect(item).toHaveAttribute("aria-current", "true");

        await userEvent.keyboard("{Enter}");
        expect(
            await screen.findByRole("heading", { name: "Rust 工具链更新" }),
        ).toHaveFocus();

        await userEvent.keyboard("{Escape}");
        expect(item).toHaveFocus();
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
            .mockResolvedValueOnce({
                ...demo.items[0],
                contract_version: 1,
                dataset_id: "demo-v1",
                what_happened: "发生了什么",
                why_it_matters: "为什么重要",
                possible_impact: "可能影响",
                facts: ["原始事实"],
                rule_reasons: ["规则判断"],
                ai_content: "演示 AI 生成",
                ai_confidence_percent: 88,
                provenance: {
                    source_kind: "official_release",
                    publisher: "Rust Project",
                    author: null,
                    original_title: "Rust update",
                    original_url: "https://www.rust-lang.org/",
                    published_at: "2026-06-20T10:00:00Z",
                    collected_at: "2026-06-20T10:30:00Z",
                    first_discovered_at: "2026-06-20T10:30:00Z",
                    last_updated_at: "2026-06-20T10:30:00Z",
                    availability_status: "available",
                    deterministic_association_basis: "demo_fixture_id",
                },
            });
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

    it("loads keyset pages without duplicating existing list items", async () => {
        const secondItem = {
            ...demo.items[0],
            id: "demo:rust-002",
            title: "Rust 第二页情报",
            original_url: "https://www.rust-lang.org/learn",
        };
        const demoList = vi
            .fn()
            .mockResolvedValueOnce({
                ...demo,
                next_cursor:
                    "v1:64656d6f2d7631:-:323032362d30362d32305431303a30303a30305a:64656d6f3a727573742d303031:0123456789abcdef",
            })
            .mockResolvedValueOnce({
                ...demo,
                items: [demo.items[0], secondItem],
                next_cursor: null,
            });
        renderShell(api({ demoList }));

        await screen.findByRole("button", { name: /Rust 工具链更新/ });
        await userEvent.click(screen.getByRole("button", { name: "加载更多" }));
        expect(
            await screen.findByRole("button", { name: /Rust 第二页情报/ }),
        ).toBeVisible();
        expect(
            screen.getAllByRole("button", { name: /Rust 工具链更新/ }),
        ).toHaveLength(1);
        expect(demoList).toHaveBeenNthCalledWith(2, expect.any(String), 20);
    });

    it.each([
        ["waiting", "演示 AI 等待中"],
        ["failed", "演示 AI 失败"],
        ["unavailable", "演示 AI 不可用"],
    ] as const)(
        "renders %s AI evidence without claiming it was generated",
        async (status, heading) => {
            renderShell(
                api({
                    demoDetail: vi
                        .fn()
                        .mockResolvedValue(detailFor(demo.items[0], status)),
                }),
            );
            expect(
                await screen.findByRole("heading", { name: heading }),
            ).toBeVisible();
            expect(
                screen.queryByRole("heading", { name: "演示 AI 生成" }),
            ).not.toBeInTheDocument();
        },
    );

    it("moves selection to a valid item without stealing focus from the active search control", async () => {
        const replacement = {
            ...demo.items[0],
            id: "demo:replacement",
            title: "替换后的情报",
            original_url: "https://example.com/replacement",
        };
        renderShell(
            api({
                demoSearch: vi
                    .fn()
                    .mockResolvedValue({ ...demo, items: [replacement] }),
                demoDetail: vi.fn((id) =>
                    Promise.resolve(
                        detailFor(
                            id === replacement.id ? replacement : demo.items[0],
                        ),
                    ),
                ),
            }),
        );
        const original = await screen.findByRole("button", {
            name: /Rust 工具链更新/,
        });
        original.focus();
        await userEvent.type(
            screen.getByRole("textbox", { name: "搜索" }),
            "替换",
        );
        const submit = screen.getByRole("button", { name: "搜索" });
        await userEvent.click(submit);

        const next = await screen.findByRole("button", {
            name: /替换后的情报/,
        });
        expect(submit).toHaveFocus();
        expect(next).toHaveAttribute("aria-current", "true");
        expect(screen.getByText(/已无原选中项/)).toBeVisible();
    });

    it("keeps existing rows visible when loading another page fails", async () => {
        const demoList = vi
            .fn()
            .mockResolvedValueOnce({
                ...demo,
                next_cursor:
                    "v1:64656d6f2d7631:-:323032362d30362d32305431303a30303a30305a:64656d6f3a727573742d303031:0123456789abcdef",
            })
            .mockRejectedValueOnce(new Error("private page failure"));
        renderShell(api({ demoList }));
        const original = await screen.findByRole("button", {
            name: /Rust 工具链更新/,
        });
        await userEvent.click(screen.getByRole("button", { name: "加载更多" }));

        expect(original).toBeVisible();
        expect(await screen.findByText(/继续显示上次可用内容/)).toBeVisible();
        expect(
            screen.queryByText("private page failure"),
        ).not.toBeInTheDocument();
    });

    it("never renders a late detail response for the previously selected item", async () => {
        let resolveFirst!: (value: DemoEvidenceDetailV1) => void;
        let resolveSecond!: (value: DemoEvidenceDetailV1) => void;
        const first = new Promise<DemoEvidenceDetailV1>((resolve) => {
            resolveFirst = resolve;
        });
        const second = new Promise<DemoEvidenceDetailV1>((resolve) => {
            resolveSecond = resolve;
        });
        renderShell(
            api({
                demoBootstrap: vi.fn().mockResolvedValue({
                    ...demo,
                    items: [demo.items[0], secondDemoItem],
                }),
                demoList: vi.fn().mockResolvedValue({
                    ...demo,
                    items: [demo.items[0], secondDemoItem],
                    next_cursor: null,
                }),
                demoDetail: vi.fn((id) =>
                    id === secondDemoItem.id ? second : first,
                ),
            }),
        );
        await userEvent.click(
            await screen.findByRole("button", { name: /Rust 第二条情报/ }),
        );
        resolveSecond(detailFor(secondDemoItem));
        expect(
            await screen.findByRole("heading", { name: "Rust 第二条情报" }),
        ).toBeVisible();
        resolveFirst(detailFor());
        expect(
            screen.queryByRole("heading", { name: "Rust 工具链更新" }),
        ).not.toBeInTheDocument();
    });

    it("resets only the detail scroll container when selection changes", async () => {
        renderShell(
            api({
                demoBootstrap: vi.fn().mockResolvedValue({
                    ...demo,
                    items: [demo.items[0], secondDemoItem],
                }),
                demoList: vi.fn().mockResolvedValue({
                    ...demo,
                    items: [demo.items[0], secondDemoItem],
                    next_cursor: null,
                }),
                demoDetail: vi.fn((id) =>
                    Promise.resolve(
                        detailFor(
                            id === secondDemoItem.id
                                ? secondDemoItem
                                : demo.items[0],
                        ),
                    ),
                ),
            }),
        );
        await screen.findByRole("heading", { name: "Rust 工具链更新" });
        const detailRegion = screen.getByRole("region", {
            name: "演示情报详情",
        });
        detailRegion.scrollTop = 240;
        await userEvent.click(
            screen.getByRole("button", { name: /Rust 第二条情报/ }),
        );
        await screen.findByRole("heading", { name: "Rust 第二条情报" });
        expect(detailRegion.scrollTop).toBe(0);
    });
});
