import {
    createSyncResultPage,
    encodeSyncResultCursor,
} from "../support/factories/demo-dto.factory";
import {
    commandError,
    expect,
    test,
} from "../support/fixtures/demo-app.fixture";

const RUN_ID = "run:0123456789abcdef01234567";

test.describe("Story 2.6 RSS/Atom sync results", () => {
    test("[P0] 明确 run 可重载并且只调用结果查询", async ({
        page,
        demoApp,
    }) => {
        await page.goto(`/sync/${RUN_ID}`);

        await expect(
            page.getByRole("heading", { name: "同步成功，已获得新结果" }),
        ).toBeVisible();
        await expect(page.getByText("Rust release")).toBeVisible();
        await expect(page.getByText("example.com · RSS/Atom")).toBeVisible();
        expect(await demoApp.invokeCalls()).toEqual([
            {
                command: "get_sync_result_v1",
                args: {
                    input: {
                        contract_version: 1,
                        sync_run_id: RUN_ID,
                        cursor: null,
                        limit: 25,
                    },
                },
            },
        ]);

        await page.reload();
        await expect(page.getByText("Rust release")).toBeVisible();
        expect(await demoApp.invokeCalls()).toEqual([
            {
                command: "get_sync_result_v1",
                args: {
                    input: {
                        contract_version: 1,
                        sync_run_id: RUN_ID,
                        cursor: null,
                        limit: 25,
                    },
                },
            },
        ]);
        expect(await demoApp.externalCalls()).toEqual([]);
    });

    test("[P0] partial 保留零结果成功来源并显示失败范围", async ({
        page,
        demoApp,
    }) => {
        const base = createSyncResultPage();
        await demoApp.setResponse(
            "get_sync_result_v1",
            createSyncResultPage({
                summary: {
                    ...base.summary,
                    outcome: "partially_succeeded",
                    counts: {
                        inserted: 0,
                        updated: 0,
                        skipped: 1,
                        failed: 1,
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
            }),
        );
        await page.goto(`/sync/${RUN_ID}`);
        await expect(
            page.getByRole("heading", {
                name: "部分来源成功，已保留确认结果",
            }),
        ).toBeVisible();
        await expect(page.getByText("失败范围：network.source")).toBeVisible();
        expect(await demoApp.externalCalls()).toEqual([]);
    });

    test("[P1] 零结果与全部失败使用互斥文案", async ({ page, demoApp }) => {
        const base = createSyncResultPage();
        const zero = createSyncResultPage({
            summary: {
                ...base.summary,
                outcome: "succeeded_zero_results",
                counts: { inserted: 0, updated: 0, skipped: 1, failed: 0 },
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
        });
        await demoApp.setResponse("get_sync_result_v1", zero);
        await page.goto(`/sync/${RUN_ID}`);
        await expect(
            page.getByRole("heading", { name: "同步完成，本轮没有新候选" }),
        ).toBeVisible();

        const failed = createSyncResultPage({
            summary: {
                ...base.summary,
                outcome: "failed",
                counts: { inserted: 0, updated: 0, skipped: 0, failed: 1 },
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
        });
        await demoApp.setResponse("get_sync_result_v1", failed);
        await page.reload();
        await expect(
            page.getByRole("heading", { name: "本轮同步失败" }),
        ).toBeVisible();
        await expect(
            page.getByText("该来源同步失败，没有可显示的成功结果。"),
        ).toBeVisible();
    });

    test("[P1] run-bound cursor 读取下一页", async ({ page, demoApp }) => {
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
        await demoApp.setResponse("get_sync_result_v1", {
            __syncResultState: "read",
            initialPages: [first, second],
        });
        await page.goto(`/sync/${RUN_ID}`);
        await page.getByRole("button", { name: "加载更多" }).click();
        await expect(page.getByText("Second page result")).toBeVisible();
        expect(
            (await demoApp.invokeCalls()).map((call) => call.command),
        ).toEqual(["get_sync_result_v1", "get_sync_result_v1"]);
    });

    test.describe("initial error", () => {
        test.use({
            tauriCommandOverrides: {
                get_sync_result_v1: commandError({
                    contract_version: 1,
                    code: "storage.source",
                    category: "storage",
                    message_key: "error.storage.source",
                    retryability: "after_user_action",
                    source_id: null,
                    task_id: null,
                    details_allowlisted: "redacted",
                    correlation_id: "corr-story-2-6",
                }),
            },
        });

        test("[P1] 首屏错误可切换为稳定响应后重试", async ({
            page,
            demoApp,
        }) => {
            await page.goto(`/sync/${RUN_ID}`);
            await expect(page.getByRole("alert")).toContainText(
                "本轮结果暂时不可用",
            );
            await demoApp.setResponse(
                "get_sync_result_v1",
                createSyncResultPage(),
            );
            await page.getByRole("button", { name: "重试" }).click();
            await expect(page.getByText("Rust release")).toBeVisible();
            expect(await demoApp.externalCalls()).toEqual([]);
        });
    });
});
