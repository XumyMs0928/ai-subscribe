import { expect, test } from "../support/fixtures";
import {
    createIntelFeedItem,
    createIntelFeedPage,
} from "../support/factories/demo-dto.factory";

test.describe("Story 4.5 real RSS intelligence feed", () => {
    test("[P0] high-value and ordinary streams are both reachable without external calls", async ({
        page,
        demoApp,
    }) => {
        await page.goto("/");

        await expect(page.getByRole("heading", { name: "情报" })).toBeVisible();
        const feedItem = page.getByRole("button", {
            name: /AI agent security release/,
        });
        await expect(feedItem).toContainText("高价值");
        await expect(
            page.getByRole("link", { name: /演示数据/ }),
        ).toHaveAttribute("href", "/demo");

        await page.getByRole("button", { name: "普通候选" }).click();
        await expect(
            page.getByRole("button", { name: /Quarterly community note/ }),
        ).toContainText("普通候选");

        const commands = (await demoApp.invokeCalls()).map(
            (call) => call.command,
        );
        expect(new Set(commands)).toEqual(
            new Set(["query_intel_feed_v1", "sync_health_v1"]),
        );
        await expect.poll(() => demoApp.externalCalls()).toEqual([]);
    });

    test("[P0] four-dimensional filters remain a narrow core query", async ({
        page,
        demoApp,
    }) => {
        await page.goto("/");
        await expect(page.getByText("AI agent security release")).toBeVisible();

        await page.getByLabel("赛道 ID").fill("ai_agents");
        await page
            .getByLabel("来源 ID")
            .fill("source:111111111111111111111111");
        await page.getByLabel("时间范围").selectOption("last_7d");
        await page.getByRole("checkbox", { name: "高" }).check();
        await page.getByRole("checkbox", { name: "中" }).check();
        await page.getByRole("button", { name: "应用筛选" }).click();

        await expect(
            page.getByText("已应用筛选", { exact: false }),
        ).toBeVisible();
        await expect
            .poll(async () => {
                const calls = await demoApp.invokeCalls();
                return calls.findLast(
                    (call) => call.command === "query_intel_feed_v1",
                )?.args;
            })
            .toEqual({
                input: {
                    contract_version: 1,
                    stream: "high_value",
                    filters: {
                        track_ids: ["ai_agents"],
                        source_ids: ["source:111111111111111111111111"],
                        time_window: "last_7d",
                        importance: ["high", "medium"],
                    },
                    sort: "score_desc",
                    cursor: null,
                    limit: 30,
                },
            });
        await expect(page).toHaveURL(/tracks=ai_agents/);
        await page.reload();
        await expect(page.getByText("AI agent security release")).toBeVisible();
        await expect
            .poll(async () => {
                const calls = await demoApp.invokeCalls();
                return calls.findLast(
                    (call) => call.command === "query_intel_feed_v1",
                )?.args;
            })
            .toEqual({
                input: expect.objectContaining({
                    filters: expect.objectContaining({
                        track_ids: ["ai_agents"],
                        source_ids: ["source:111111111111111111111111"],
                        time_window: "last_7d",
                        importance: ["high", "medium"],
                    }),
                }),
            });
        await expect.poll(() => demoApp.externalCalls()).toEqual([]);
    });

    test("[P0] cursor pagination and navigation preserve the current real-feed context", async ({
        page,
        demoApp,
    }) => {
        const items = Array.from({ length: 31 }, (_, index) =>
            createIntelFeedItem({
                intel_item_id: `intel:${(index + 1)
                    .toString(16)
                    .padStart(64, "0")}`,
                title: `Paged RSS item ${index + 1}`,
                score: 100 - index,
            }),
        );
        await demoApp.setResponse("query_intel_feed_v1", {
            __intelFeedState: "read",
            initialPage: createIntelFeedPage({ items }),
        });
        await page.goto("/");
        const first = page.getByRole("button", {
            name: /Paged RSS item 1(?:\s|$)/,
        });
        await expect(first).toBeVisible();
        await first.focus();
        await page.keyboard.press("ArrowDown");
        const selected = page.getByRole("button", {
            name: /Paged RSS item 2(?:\s|$)/,
        });
        await expect(selected).toHaveAttribute("aria-pressed", "true");

        await page.getByRole("button", { name: "加载更多" }).click();
        await expect(page.getByText("Paged RSS item 31")).toBeVisible();
        await expect
            .poll(async () => {
                const calls = await demoApp.invokeCalls();
                return calls.some((call) => {
                    const cursor = (
                        call.args as { input?: { cursor?: string | null } }
                    ).input?.cursor;
                    return (
                        call.command === "query_intel_feed_v1" &&
                        typeof cursor === "string"
                    );
                });
            })
            .toBe(true);

        await page.getByRole("link", { name: "来源" }).click();
        await page.getByRole("link", { name: "情报" }).click();
        await expect(selected).toHaveAttribute("aria-pressed", "true");
        await expect.poll(() => demoApp.externalCalls()).toEqual([]);
    });
});
