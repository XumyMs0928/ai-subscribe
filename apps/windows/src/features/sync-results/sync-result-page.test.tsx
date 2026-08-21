import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router";
import { describe, expect, test, vi } from "vitest";

import { DesktopApiProvider } from "../../app/providers/desktop-api-provider";
import type { DesktopApi } from "../../lib/desktop-api/desktop-api";
import { createAppQueryClient } from "../../lib/query-client";
import {
    createSyncResultPage,
    encodeSyncResultCursor,
} from "../../../../../tests/support/factories/demo-dto.factory";
import { SyncResultPage } from "./sync-result-page";

const RUN_ID = "run:0123456789abcdef01234567";

function api(getSyncResult: DesktopApi["getSyncResult"]): DesktopApi {
    return {
        health: vi.fn(),
        demoBootstrap: vi.fn(),
        demoSearch: vi.fn(),
        demoList: vi.fn(),
        demoFilter: vi.fn(),
        demoDetail: vi.fn(),
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
        getSyncResult,
        queryIntelFeed: vi.fn(),
        queryIntelEvidenceDetail: vi.fn(),
        openIntelOriginal: vi.fn(),
    };
}

function renderPage(desktopApi: DesktopApi, path = `/sync/${RUN_ID}`) {
    render(
        <QueryClientProvider client={createAppQueryClient()}>
            <DesktopApiProvider api={desktopApi}>
                <MemoryRouter initialEntries={[path]}>
                    <Routes>
                        <Route
                            path="/sync/:syncRunId"
                            element={<SyncResultPage />}
                        />
                    </Routes>
                </MemoryRouter>
            </DesktopApiProvider>
        </QueryClientProvider>,
    );
}

describe("SyncResultPage", () => {
    test("按冻结 source_id 分组多个成功来源的结果", async () => {
        const base = createSyncResultPage();
        const secondSourceId = "source:fedcba9876543210fedcba98";
        const page = createSyncResultPage({
            summary: {
                ...base.summary,
                counts: { inserted: 2, updated: 0, skipped: 0, failed: 0 },
                sources: [
                    base.summary.sources[0],
                    {
                        ...base.summary.sources[0],
                        source_id: secondSourceId,
                    },
                ],
            },
            items: [
                base.items[0],
                {
                    ...base.items[0],
                    result_item_id: "result:fedcba9876543210fedcba98",
                    source_id: secondSourceId,
                    original_title: "Second source release",
                },
            ],
        });
        renderPage(api(vi.fn().mockResolvedValue(page)));

        const firstGroup = await screen.findByTestId(
            `source-result-${base.summary.sources[0].source_id}`,
        );
        const secondGroup = screen.getByTestId(
            `source-result-${secondSourceId}`,
        );
        expect(within(firstGroup).getByText("Rust release")).toBeVisible();
        expect(
            within(secondGroup).getByText("Second source release"),
        ).toBeVisible();
        expect(
            within(firstGroup).queryByText("Second source release"),
        ).toBeNull();
    });

    test("部分成功允许成功来源零结果并保留失败范围", async () => {
        const base = createSyncResultPage();
        const page = createSyncResultPage({
            summary: {
                ...base.summary,
                outcome: "partially_succeeded",
                counts: { inserted: 0, updated: 0, skipped: 1, failed: 1 },
                sources: [
                    {
                        ...base.summary.sources[0],
                        counts: {
                            inserted: 0,
                            updated: 0,
                            skipped: 1,
                            failed: 0,
                        },
                    },
                    {
                        ...base.summary.sources[0],
                        source_id: "source:fedcba9876543210fedcba98",
                        publisher: "failed.example",
                        status: "failed",
                        counts: {
                            inserted: 0,
                            updated: 0,
                            skipped: 0,
                            failed: 1,
                        },
                        error_code: "network.source",
                    },
                ],
            },
            items: [],
        });
        renderPage(api(vi.fn().mockResolvedValue(page)));
        expect(
            await screen.findByText("部分来源成功，已保留确认结果"),
        ).toBeVisible();
        expect(
            screen.getByText("该来源已完成，但本轮没有成功转换的新结果。"),
        ).toBeVisible();
        expect(screen.getByText("失败范围：network.source")).toBeVisible();
        expect(
            screen.getByText("该来源同步失败，没有可显示的成功结果。"),
        ).toBeVisible();
    });

    test("分页使用 core run-bound cursor 并保持 summary 一致", async () => {
        const base = createSyncResultPage();
        const summary = {
            ...base.summary,
            counts: { inserted: 2, updated: 0, skipped: 0, failed: 0 },
            sources: [
                {
                    ...base.summary.sources[0],
                    counts: {
                        inserted: 2,
                        updated: 0,
                        skipped: 0,
                        failed: 0,
                    },
                },
            ],
        } as const;
        const first = createSyncResultPage({
            summary,
            next_cursor: encodeSyncResultCursor(base.items[0]),
        });
        const second = createSyncResultPage({
            summary,
            items: [
                {
                    ...base.items[0],
                    result_item_id: "result:fedcba9876543210fedcba98",
                    original_title: "Second page result",
                },
            ],
        });
        const getSyncResult = vi
            .fn()
            .mockResolvedValueOnce(first)
            .mockResolvedValueOnce(second);
        renderPage(api(getSyncResult));
        await userEvent.click(
            await screen.findByRole("button", { name: "加载更多" }),
        );
        expect(await screen.findByText("Second page result")).toBeVisible();
        expect(getSyncResult).toHaveBeenLastCalledWith(
            expect.objectContaining({ cursor: first.next_cursor }),
        );
    });

    test("后续页失败时保留首屏并提供局部重试", async () => {
        const base = createSyncResultPage();
        const first = createSyncResultPage({
            next_cursor: encodeSyncResultCursor(base.items[0]),
        });
        const getSyncResult = vi
            .fn()
            .mockResolvedValueOnce(first)
            .mockRejectedValueOnce(new Error("redacted"))
            .mockResolvedValueOnce(
                createSyncResultPage({
                    summary: first.summary,
                    items: [
                        {
                            ...base.items[0],
                            result_item_id: "result:fedcba9876543210fedcba98",
                            original_title: "Recovered page",
                        },
                    ],
                }),
            );
        renderPage(api(getSyncResult));
        await userEvent.click(
            await screen.findByRole("button", { name: "加载更多" }),
        );
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "已显示的结果仍保留",
        );
        expect(screen.getByText("Rust release")).toBeVisible();
        await userEvent.click(screen.getByRole("button", { name: "重试读取" }));
        expect(await screen.findByText("Recovered page")).toBeVisible();
    });

    test("零结果使用成功空态而不是失败文案", async () => {
        const base = createSyncResultPage();
        renderPage(
            api(
                vi.fn().mockResolvedValue(
                    createSyncResultPage({
                        summary: {
                            ...base.summary,
                            outcome: "succeeded_zero_results",
                            counts: {
                                inserted: 0,
                                updated: 0,
                                skipped: 1,
                                failed: 0,
                            },
                            sources: [
                                {
                                    ...base.summary.sources[0],
                                    counts: {
                                        inserted: 0,
                                        updated: 0,
                                        skipped: 1,
                                        failed: 0,
                                    },
                                },
                            ],
                        },
                        items: [],
                    }),
                ),
            ),
        );
        expect(
            await screen.findByText("同步完成，本轮没有新候选"),
        ).toBeVisible();
        expect(
            screen.getByText("同步已完成，本轮没有可显示的新候选。"),
        ).toBeVisible();
        expect(screen.queryByText("本轮同步失败")).not.toBeInTheDocument();
    });

    test("全部失败空页显示失败专用文案", async () => {
        const base = createSyncResultPage();
        renderPage(
            api(
                vi.fn().mockResolvedValue(
                    createSyncResultPage({
                        summary: {
                            ...base.summary,
                            outcome: "failed",
                            counts: {
                                inserted: 0,
                                updated: 0,
                                skipped: 0,
                                failed: 1,
                            },
                            sources: [
                                {
                                    ...base.summary.sources[0],
                                    status: "failed",
                                    counts: {
                                        inserted: 0,
                                        updated: 0,
                                        skipped: 0,
                                        failed: 1,
                                    },
                                    error_code: "network.source",
                                },
                            ],
                        },
                        items: [],
                    }),
                ),
            ),
        );
        expect(await screen.findByText("本轮同步失败")).toBeVisible();
        expect(
            screen.getByText("该来源同步失败，没有可显示的成功结果。"),
        ).toBeVisible();
        expect(
            screen.queryByText("同步已完成，本轮没有可显示的新候选。"),
        ).not.toBeInTheDocument();
    });

    test("重复 cursor 的后续页不会被拼接", async () => {
        const base = createSyncResultPage();
        const cursor = encodeSyncResultCursor(base.items[0]);
        const first = createSyncResultPage({ next_cursor: cursor });
        const second = createSyncResultPage({
            summary: first.summary,
            items: [
                {
                    ...base.items[0],
                    result_item_id: "result:fedcba9876543210fedcba98",
                    original_title: "Rejected repeated cursor page",
                },
            ],
            next_cursor: cursor,
        });
        renderPage(
            api(
                vi
                    .fn()
                    .mockResolvedValueOnce(first)
                    .mockResolvedValueOnce(second),
            ),
        );
        await userEvent.click(
            await screen.findByRole("button", { name: "加载更多" }),
        );
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "已停止拼接",
        );
        expect(
            screen.queryByText("Rejected repeated cursor page"),
        ).not.toBeInTheDocument();
        expect(screen.getByText("Rust release")).toBeVisible();
    });

    test("后续页 summary 漂移时保留首页", async () => {
        const base = createSyncResultPage();
        const first = createSyncResultPage({
            next_cursor: encodeSyncResultCursor(base.items[0]),
        });
        const second = createSyncResultPage({
            summary: {
                ...first.summary,
                task_id: "task:fedcba9876543210fedcba98",
            },
            items: [
                {
                    ...base.items[0],
                    result_item_id: "result:fedcba9876543210fedcba98",
                    original_title: "Rejected summary page",
                },
            ],
        });
        renderPage(
            api(
                vi
                    .fn()
                    .mockResolvedValueOnce(first)
                    .mockResolvedValueOnce(second),
            ),
        );
        await userEvent.click(
            await screen.findByRole("button", { name: "加载更多" }),
        );
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "本轮身份不一致",
        );
        expect(screen.queryByText("Rejected summary page")).toBeNull();
    });

    test("非法 run 路由直接失败且不调用 DesktopApi", () => {
        const getSyncResult = vi.fn();
        renderPage(api(getSyncResult), "/sync/not-a-run");
        expect(screen.getByRole("alert")).toHaveTextContent("标识无效");
        expect(screen.getByRole("link", { name: "返回来源" })).toBeVisible();
        expect(getSyncResult).not.toHaveBeenCalled();
        expect(screen.queryByText("正在读取本轮结果…")).toBeNull();
    });

    test("首屏错误可重试并恢复", async () => {
        const getSyncResult = vi
            .fn()
            .mockRejectedValueOnce(new Error("redacted"))
            .mockResolvedValueOnce(createSyncResultPage());
        renderPage(api(getSyncResult));
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "本轮结果暂时不可用",
        );
        await userEvent.click(screen.getByRole("button", { name: "重试" }));
        expect(
            await screen.findByRole("heading", {
                name: "同步成功，已获得新结果",
            }),
        ).toBeVisible();
    });
});
