import type { Page } from "@playwright/test";

import { expect, test } from "../support/fixtures/demo-app.fixture";
import {
    createSourceDeliveryReadiness,
    createSourcePage,
    createSourceReadiness,
    createSourceSyncStatus,
    createSourceView,
    createSyncHealthSummary,
    createTaskSnapshot,
} from "../support/factories/demo-dto.factory";

async function saveSource(page: Page, url: string) {
    await page.getByLabel("公开 HTTPS Feed 地址").fill(url);
    await page.getByRole("button", { name: "添加来源" }).click();
    await expect(page.getByText(/^已保存 /)).toBeVisible();
}

test.describe("Story 2.5 RSS/Atom synchronization", () => {
    test("[P0] 同步全部 RSS/Atom 后显示终态并在 reload 后恢复任务状态", async ({
        page,
        demoApp,
    }) => {
        await page.goto("/sources");
        await saveSource(page, "https://example.com/primary.xml");

        await page.getByRole("button", { name: "同步全部 RSS/Atom" }).click();
        await expect(page.getByTestId("latest-sync-task")).toContainText(
            "同步成功",
        );

        const calls = await demoApp.invokeCalls();
        const start = calls.find((call) => call.command === "start_sync_v1");
        expect(start?.args?.input).toMatchObject({
            contract_version: 1,
            target: { kind: "all_enabled_rss_atom" },
            foreground_budget_ms: 30_000,
        });
        expect(calls.some((call) => call.command === "task_v1")).toBeTruthy();

        await page.reload();
        await expect(page.getByTestId("latest-sync-task")).toContainText(
            "同步成功",
        );
        await expect(page.getByText("Windows RSS 最小闭环就绪")).toBeVisible();
        expect(await demoApp.externalCalls()).toEqual([]);
        await expect(page.getByText(/GitHub|arXiv|三来源已就绪/)).toHaveCount(
            0,
        );
    });

    test("[P0] 立即同步只把选中的 RSS/Atom source_id 交给 DesktopApi", async ({
        page,
        demoApp,
    }) => {
        await page.goto("/sources");
        await saveSource(page, "https://example.com/single.xml");

        await page.getByRole("button", { name: "立即同步" }).click();
        await expect(page.getByTestId("latest-sync-task")).toContainText(
            "同步成功",
        );

        const start = (await demoApp.invokeCalls()).find(
            (call) => call.command === "start_sync_v1",
        );
        expect(start?.args?.input).toMatchObject({
            target: {
                kind: "source_id",
                source_id: expect.stringMatching(/^source:[0-9a-f]{24}$/),
            },
        });
        expect(await demoApp.externalCalls()).toEqual([]);
    });

    test("[P1] 全量同步部分失败时保留成功和失败来源且不折叠为整体失败", async ({
        page,
        demoApp,
    }) => {
        const success = createSourceView({
            source_id: "source:bbbbbbbbbbbbbbbbbbbbbbbb",
            display_url: "https://example.com/success.xml",
        });
        const failed = createSourceView({
            source_id: "source:aaaaaaaaaaaaaaaaaaaaaaaa",
            display_url: "https://example.com/failed.xml",
        });
        const successResult = createSourceSyncStatus({
            source_id: success.source_id,
        });
        const failedResult = createSourceSyncStatus({
            source_id: failed.source_id,
            state: "failed",
            last_success_at: null,
            error_code: "source_format.rss_atom",
        });
        await page.goto("/sources");
        await demoApp.setResponse("query_sources_v1", {
            __sourceState: "query",
            initialPage: createSourcePage({ items: [success, failed] }),
        });
        await demoApp.setResponse("task_v1", {
            __syncState: "task",
            initialTask: createTaskSnapshot({
                state: "partially_succeeded",
                sources: [successResult, failedResult],
                error_summary: "source_format.rss_atom",
            }),
        });
        await page.reload();

        await page.getByRole("button", { name: "同步全部 RSS/Atom" }).click();

        await expect(
            page.getByText(/部分来源同步成功，其他 RSS\/Atom 来源仍需处理/),
        ).toBeVisible();
        await expect(page.getByText(success.display_url)).toBeVisible();
        await expect(page.getByText(failed.display_url)).toBeVisible();
        await expect(
            page.getByText("来源错误：source_format.rss_atom"),
        ).toBeVisible();
        expect(await demoApp.externalCalls()).toEqual([]);
    });

    test("[P1] Retry-After 截止前禁用单源同步并显示可恢复原因", async ({
        page,
        demoApp,
    }) => {
        await page.clock.setFixedTime(new Date("2026-08-18T02:00:00.000Z"));
        const retryAt = "2099-08-18T03:00:00.000Z";
        const source = createSourceView({
            status: "retry_wait",
            retryability: "after",
            next_allowed_at: retryAt,
        });
        await page.goto("/sources");
        await demoApp.setResponse("query_sources_v1", {
            __sourceState: "query",
            initialPage: createSourcePage({ items: [source] }),
        });
        await demoApp.setResponse("sync_health_v1", {
            __syncState: "health",
            initialHealth: createSyncHealthSummary({
                source_results: [
                    createSourceSyncStatus({
                        state: "retry_wait",
                        next_allowed_at: retryAt,
                        error_code: "rate_limit.source",
                    }),
                ],
                readiness: createSourceDeliveryReadiness({
                    status: "blocked",
                    sources: [
                        createSourceReadiness({
                            status: "retry_wait",
                            next_allowed_at: retryAt,
                        }),
                    ],
                }),
            }),
        });
        await page.reload();

        await expect(
            page.getByRole("button", { name: "立即同步" }),
        ).toBeDisabled();
        await expect(
            page.getByText(`服务端要求等待至 ${retryAt}。`),
        ).toBeVisible();
        expect(await demoApp.externalCalls()).toEqual([]);
    });
});
