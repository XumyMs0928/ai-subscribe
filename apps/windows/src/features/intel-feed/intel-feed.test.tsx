import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, test, vi } from "vitest";

import { DesktopApiProvider } from "../../app/providers/desktop-api-provider";
import type {
    DesktopApi,
    IntelFeedPageV1,
    QueryIntelFeedInputV1,
} from "../../lib/desktop-api/desktop-api";
import { createAppQueryClient } from "../../lib/query-client";
import { IntelFeed } from "./intel-feed";

const item = {
    contract_version: 1,
    intel_item_id: `intel:${"1".repeat(64)}`,
    source_id: "source:111111111111111111111111",
    source_kind: "rss_atom",
    publisher: "publisher.example",
    title: "AI agent security release",
    source_excerpt: "A bounded source excerpt.",
    excerpt_truncated: false,
    published_at: "2026-08-20T08:00:00Z",
    collected_at: "2026-08-20T08:05:00Z",
    importance: "high",
    score: 95,
    matched_track_ids: ["ai_agents"],
    stream_disposition: "high_value",
    ai_status: "unavailable",
} as const;

afterEach(() => {
    vi.restoreAllMocks();
});

function page(stream: "high_value" | "ordinary_candidate"): IntelFeedPageV1 {
    return {
        contract_version: 1,
        stream,
        filters: {
            track_ids: [],
            source_ids: [],
            time_window: "all_time",
            importance: [],
        },
        sort: "score_desc",
        rule_version: "rss-intelligence-value-v1",
        configuration_revision: 1,
        configuration_hash: "2".repeat(64),
        as_of_ms: 1_777_000_000_000,
        items: [{ ...item, stream_disposition: stream }],
        next_cursor: null,
    };
}

function renderFeed(
    queryIntelFeed: DesktopApi["queryIntelFeed"],
    initialEntries: readonly string[] = ["/"],
) {
    const desktopApi = {
        queryIntelFeed,
        syncHealth: vi.fn().mockResolvedValue({
            contract_version: 1,
            readiness: {
                contract_version: 1,
                required_source_kinds: ["rss_atom"],
                status: "ready",
                sources: [
                    {
                        contract_version: 1,
                        source_id: "source:111111111111111111111111",
                        source_kind: "rss_atom",
                        status: "available",
                        last_success_at: "2026-08-20T08:05:00Z",
                        next_allowed_at: null,
                    },
                ],
            },
            source_results: [],
            latest_task: null,
            pending_task_count: 0,
            last_success_at: "2026-08-20T08:05:00Z",
            freshness: "fresh",
        }),
    } as unknown as DesktopApi;
    render(
        <QueryClientProvider client={createAppQueryClient()}>
            <DesktopApiProvider api={desktopApi}>
                <MemoryRouter initialEntries={[...initialEntries]}>
                    <IntelFeed />
                </MemoryRouter>
            </DesktopApiProvider>
        </QueryClientProvider>,
    );
}

describe("IntelFeed", () => {
    test("默认显示真实 RSS 高价值主流", async () => {
        renderFeed(vi.fn().mockResolvedValue(page("high_value")));

        const feedItem = await screen.findByRole("button", {
            name: /AI agent security release/,
        });
        expect(within(feedItem).getByText("高价值")).toBeVisible();
        expect(screen.queryByText("收藏")).not.toBeInTheDocument();
    });

    test("普通候选入口可达且不会删除候选", async () => {
        const query = vi
            .fn()
            .mockImplementation((input: QueryIntelFeedInputV1) =>
                Promise.resolve(page(input.stream)),
            );
        renderFeed(query);
        await screen.findByText(item.title);

        await userEvent.click(screen.getByRole("button", { name: "普通候选" }));

        const candidateItem = await screen.findByRole("button", {
            name: /AI agent security release/,
        });
        expect(within(candidateItem).getByText("普通候选")).toBeVisible();
        expect(query).toHaveBeenLastCalledWith(
            expect.objectContaining({ stream: "ordinary_candidate" }),
        );
    });

    test("组合筛选由 DesktopApi 发送，不在组件内重算", async () => {
        const query = vi
            .fn()
            .mockImplementation((input: QueryIntelFeedInputV1) =>
                Promise.resolve({
                    ...page(input.stream),
                    filters: input.filters,
                }),
            );
        renderFeed(query);
        await screen.findByText(item.title);

        await userEvent.type(screen.getByLabelText("赛道 ID"), "ai_agents");
        await userEvent.type(
            screen.getByLabelText("来源 ID"),
            "source:111111111111111111111111",
        );
        await userEvent.selectOptions(
            screen.getByLabelText("时间范围"),
            "last_7d",
        );
        await userEvent.click(screen.getByRole("checkbox", { name: "高" }));
        await userEvent.click(screen.getByRole("checkbox", { name: "中" }));
        await userEvent.click(screen.getByRole("button", { name: "应用筛选" }));

        expect(query).toHaveBeenLastCalledWith(
            expect.objectContaining({
                filters: {
                    track_ids: ["ai_agents"],
                    source_ids: ["source:111111111111111111111111"],
                    time_window: "last_7d",
                    importance: ["high", "medium"],
                },
            }),
        );
    });

    test("方向键移动选择并保持可见焦点", async () => {
        const second = {
            ...item,
            intel_item_id: `intel:${"3".repeat(64)}`,
            title: "Second RSS item",
            score: 90,
        };
        renderFeed(
            vi.fn().mockResolvedValue({
                ...page("high_value"),
                items: [item, second],
            }),
        );
        const first = await screen.findByRole("button", {
            name: /AI agent security release/,
        });
        const next = screen.getByRole("button", { name: /Second RSS item/ });

        first.focus();
        await userEvent.keyboard("{ArrowDown}");

        expect(next).toHaveFocus();
        expect(next).toHaveAttribute("aria-pressed", "true");
    });

    test("首载和空结果提供持久、可恢复状态", async () => {
        let resolvePage: ((value: IntelFeedPageV1) => void) | undefined;
        const query = vi.fn().mockImplementation(
            () =>
                new Promise<IntelFeedPageV1>((resolve) => {
                    resolvePage = resolve;
                }),
        );
        renderFeed(query);
        expect(
            screen.getByRole("status", { name: "正在加载情报" }),
        ).toBeVisible();

        resolvePage?.({ ...page("high_value"), items: [] });

        expect(
            await screen.findByText("当前条件下没有高价值情报。"),
        ).toBeVisible();
        expect(
            screen.getAllByRole("button", { name: "恢复默认" }).length,
        ).toBeGreaterThan(0);
    });

    test("下一页失败保留已有内容并提供局部重试", async () => {
        const query = vi
            .fn()
            .mockImplementation((input: QueryIntelFeedInputV1) => {
                if (input.cursor !== null)
                    return Promise.reject(new Error("page failed"));
                return Promise.resolve({
                    ...page("high_value"),
                    next_cursor: `feed-v1:aa:${"1".repeat(64)}`,
                });
            });
        renderFeed(query);
        await screen.findByText(item.title);

        await userEvent.click(screen.getByRole("button", { name: "加载更多" }));

        expect(await screen.findByText(/下一页加载失败/)).toBeVisible();
        expect(screen.getByText(item.title)).toBeVisible();
        expect(
            screen.getByRole("button", { name: "重试下一页" }),
        ).toBeVisible();
    });

    test("合法下一页保持 projection identity 并追加新条目", async () => {
        const cursor = `feed-v1:aa:${"1".repeat(64)}`;
        const second = {
            ...item,
            intel_item_id: `intel:${"3".repeat(64)}`,
            title: "Second RSS item",
            score: 90,
        };
        const query = vi
            .fn()
            .mockImplementation((input: QueryIntelFeedInputV1) =>
                Promise.resolve(
                    input.cursor === null
                        ? { ...page("high_value"), next_cursor: cursor }
                        : {
                              ...page("high_value"),
                              items: [second],
                              next_cursor: null,
                          },
                ),
            );
        renderFeed(query);
        await screen.findByText(item.title);

        await userEvent.click(screen.getByRole("button", { name: "加载更多" }));

        expect(await screen.findByText(second.title)).toBeVisible();
        expect(query).toHaveBeenLastCalledWith(
            expect.objectContaining({ cursor }),
        );
    });

    test("跨页 projection metadata 漂移 fail closed 且保留首屏", async () => {
        const cursor = `feed-v1:aa:${"1".repeat(64)}`;
        const query = vi
            .fn()
            .mockImplementation((input: QueryIntelFeedInputV1) =>
                Promise.resolve(
                    input.cursor === null
                        ? { ...page("high_value"), next_cursor: cursor }
                        : {
                              ...page("high_value"),
                              configuration_revision: 2,
                              items: [
                                  {
                                      ...item,
                                      intel_item_id: `intel:${"3".repeat(64)}`,
                                      title: "Drifted RSS item",
                                  },
                              ],
                          },
                ),
            );
        renderFeed(query);
        await screen.findByText(item.title);
        await userEvent.click(screen.getByRole("button", { name: "加载更多" }));

        expect(await screen.findByText(/下一页加载失败/)).toBeVisible();
        expect(screen.getByText(item.title)).toBeVisible();
        expect(screen.queryByText("Drifted RSS item")).not.toBeInTheDocument();
    });

    test("失效 cursor 从首屏恢复而不是重复提交旧 cursor", async () => {
        const cursor = `feed-v1:aa:${"1".repeat(64)}`;
        const cursorError = Object.assign(new Error("stale cursor"), {
            code: "validation.source",
        });
        const query = vi
            .fn()
            .mockImplementation((input: QueryIntelFeedInputV1) =>
                input.cursor === null
                    ? Promise.resolve({
                          ...page("high_value"),
                          next_cursor: cursor,
                      })
                    : Promise.reject(cursorError),
            );
        renderFeed(query);
        await screen.findByText(item.title);
        await userEvent.click(screen.getByRole("button", { name: "加载更多" }));

        const restart = await screen.findByRole("button", {
            name: "游标已失效，重新加载首屏",
        });
        await userEvent.click(restart);

        expect(query).toHaveBeenLastCalledWith(
            expect.objectContaining({ cursor: null }),
        );
        expect(screen.getByText(item.title)).toBeVisible();
    });

    test("无效筛选在表单层定位且不调用 DesktopApi", async () => {
        const query = vi.fn().mockResolvedValue(page("high_value"));
        renderFeed(query);
        await screen.findByText(item.title);

        await userEvent.type(screen.getByLabelText("赛道 ID"), "bad track");
        await userEvent.click(screen.getByRole("button", { name: "应用筛选" }));

        expect(screen.getByRole("alert")).toHaveTextContent("赛道 ID 只能包含");
        expect(query).toHaveBeenCalledTimes(1);
    });

    test("Enter 传递稳定 identity seam，Esc 清除提示并恢复焦点", async () => {
        renderFeed(vi.fn().mockResolvedValue(page("high_value")));
        const row = await screen.findByRole("button", {
            name: /AI agent security release/,
        });
        row.focus();

        await userEvent.keyboard("{Enter}");
        expect(screen.getByText(/证据详情将在下一阶段开放/)).toBeVisible();

        await userEvent.keyboard("{Escape}");
        expect(
            screen.queryByText(/证据详情将在下一阶段开放/),
        ).not.toBeInTheDocument();
        expect(row).toHaveFocus();
    });

    test("显式刷新移除选中项时恢复最近相邻项与焦点", async () => {
        const second = {
            ...item,
            intel_item_id: `intel:${"2".repeat(64)}`,
            title: "Second RSS item",
            score: 90,
        };
        const third = {
            ...item,
            intel_item_id: `intel:${"3".repeat(64)}`,
            title: "Third RSS item",
            score: 85,
        };
        const query = vi
            .fn()
            .mockResolvedValueOnce({
                ...page("high_value"),
                items: [item, second, third],
            })
            .mockResolvedValue({
                ...page("high_value"),
                items: [item, third],
            });
        vi.spyOn(window, "scrollBy").mockImplementation(() => undefined);
        renderFeed(query);
        const selected = await screen.findByRole("button", {
            name: /Second RSS item/,
        });
        await userEvent.click(selected);

        await userEvent.click(screen.getByRole("button", { name: "刷新情报" }));

        const replacement = await screen.findByRole("button", {
            name: /Third RSS item/,
        });
        expect(replacement).toHaveAttribute("aria-pressed", "true");
        expect(replacement).toHaveFocus();
        expect(screen.getByText(/原选中项已不在/)).toBeVisible();
    });
});
