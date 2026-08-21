import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, test, vi } from "vitest";

import { DesktopApiProvider } from "../../app/providers/desktop-api-provider";
import type { DesktopApi } from "../../lib/desktop-api/desktop-api";
import { createAppQueryClient } from "../../lib/query-client";
import {
    createConfigurationView,
    createSourcePage,
    createSourceDeliveryReadiness,
    createSourceReadiness,
    createSourceSyncStatus,
    createSourceView,
    createSyncHealthSummary,
    createTaskRef,
    createTaskSnapshot,
} from "../../../../../tests/support/factories/demo-dto.factory";
import { SourcesPage } from "./sources-page";
import { healthPollInterval, taskPollInterval } from "./sync-queries";

afterEach(() => {
    document.documentElement.scrollTop = 0;
});

function api(overrides: Partial<DesktopApi> = {}): DesktopApi {
    return {
        health: vi.fn(),
        demoBootstrap: vi.fn(),
        demoSearch: vi.fn(),
        demoList: vi.fn(),
        demoFilter: vi.fn(),
        demoDetail: vi.fn(),
        setupProgress: vi.fn(),
        saveSetupStep: vi.fn(),
        configuration: vi.fn().mockResolvedValue(createConfigurationView()),
        validateConfiguration: vi.fn(),
        saveConfiguration: vi.fn(),
        saveSource: vi.fn().mockResolvedValue(createSourceView()),
        querySources: vi
            .fn()
            .mockResolvedValue(createSourcePage({ items: [] })),
        startSync: vi.fn().mockResolvedValue(createTaskRef()),
        task: vi.fn().mockResolvedValue(createTaskSnapshot()),
        syncHealth: vi.fn().mockResolvedValue(
            createSyncHealthSummary({
                latest_task: null,
                pending_task_count: 0,
                last_success_at: null,
                freshness: null,
                source_results: [],
                readiness: createSourceDeliveryReadiness({
                    status: "not_configured",
                    sources: [],
                }),
            }),
        ),
        getSyncResult: vi.fn(),
        queryIntelFeed: vi.fn(),
        queryIntelEvidenceDetail: vi.fn(),
        openIntelOriginal: vi.fn(),
        ...overrides,
    };
}

function renderPage(desktopApi: DesktopApi) {
    render(
        <QueryClientProvider client={createAppQueryClient()}>
            <DesktopApiProvider api={desktopApi}>
                <SourcesPage />
            </DesktopApiProvider>
        </QueryClientProvider>,
    );
}

describe("SourcesPage source entry and RSS synchronization", () => {
    test("saves an HTTPS RSS source without optimistic insertion and keeps device scope visible", async () => {
        const user = userEvent.setup();
        const saveSource = vi
            .fn<DesktopApi["saveSource"]>()
            .mockResolvedValue(createSourceView());
        const querySources = vi
            .fn<DesktopApi["querySources"]>()
            .mockResolvedValueOnce(createSourcePage({ items: [] }))
            .mockResolvedValue(createSourcePage());
        renderPage(api({ saveSource, querySources }));

        expect(screen.getByText(/仅此 Windows 设备/)).toBeInTheDocument();
        await screen.findByText("尚未添加 RSS / Atom 来源。");
        await user.type(
            screen.getByLabelText("公开 HTTPS Feed 地址"),
            "https://example.com/feed.xml",
        );
        await user.click(screen.getByRole("button", { name: "添加来源" }));

        await waitFor(() => expect(saveSource).toHaveBeenCalledTimes(1));
        expect(saveSource.mock.calls[0][0]).toMatchObject({
            contract_version: 1,
            source_kind: "rss_atom",
            expected_configuration_revision: 1,
        });
        expect(
            await screen.findByText("https://example.com/feed.xml"),
        ).toBeInTheDocument();
    });

    test("save failure preserves input and returns focus to the URL field", async () => {
        const user = userEvent.setup();
        const failure = Object.assign(new Error("redacted"), {
            code: "network.source",
        });
        renderPage(api({ saveSource: vi.fn().mockRejectedValue(failure) }));
        const input = screen.getByLabelText("公开 HTTPS Feed 地址");
        await user.type(input, "https://example.com/feed.xml");
        await user.click(screen.getByRole("button", { name: "添加来源" }));
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "network.source",
        );
        expect(input).toHaveValue("https://example.com/feed.xml");
        expect(input).toHaveFocus();
    });

    test("migration failure enters an explicit read-only state and blocks source writes", async () => {
        const failure = Object.assign(new Error("redacted"), {
            code: "migration.source",
        });
        renderPage(api({ querySources: vi.fn().mockRejectedValue(failure) }));

        expect(await screen.findByRole("alert")).toHaveTextContent(
            "来源数据库升级失败，当前页面为只读",
        );
        expect(screen.getByRole("button", { name: "添加来源" })).toBeDisabled();
        expect(screen.getByRole("button", { name: "重试" })).toBeEnabled();
    });

    test("storage failure blocks source writes without masquerading as a network retry", async () => {
        const failure = Object.assign(new Error("redacted"), {
            code: "storage.source",
        });
        renderPage(api({ querySources: vi.fn().mockRejectedValue(failure) }));

        expect(await screen.findByRole("alert")).toHaveTextContent(
            "来源存储读取失败，已阻断写入",
        );
        expect(screen.getByRole("button", { name: "添加来源" })).toBeDisabled();
    });

    test("configuration failure keeps the source entry point visible but blocks saving", async () => {
        const failure = Object.assign(new Error("redacted"), {
            code: "storage.configuration",
        });
        renderPage(api({ configuration: vi.fn().mockRejectedValue(failure) }));

        expect(
            await screen.findByText(/当前设备配置暂时不可用/),
        ).toBeInTheDocument();
        expect(screen.getByLabelText("公开 HTTPS Feed 地址")).toBeVisible();
        expect(screen.getByRole("button", { name: "添加来源" })).toBeDisabled();
        expect(screen.getByRole("button", { name: "重试配置" })).toBeEnabled();
    });

    test("refresh failure preserves the previously rendered source list", async () => {
        const user = userEvent.setup();
        const querySources = vi
            .fn<DesktopApi["querySources"]>()
            .mockResolvedValueOnce(createSourcePage())
            .mockRejectedValue(
                Object.assign(new Error("redacted"), {
                    code: "network.source",
                }),
            );
        renderPage(api({ querySources }));

        expect(
            await screen.findByText("https://example.com/feed.xml"),
        ).toBeInTheDocument();
        await user.click(screen.getByRole("button", { name: "刷新" }));
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "刷新失败，保留上次来源列表",
        );
        expect(screen.getByText("https://example.com/feed.xml")).toBeVisible();
    });

    test("starts all enabled RSS sources and observes the task until its terminal state", async () => {
        const user = userEvent.setup();
        const startSync = vi
            .fn<DesktopApi["startSync"]>()
            .mockResolvedValue(createTaskRef());
        const task = vi
            .fn<DesktopApi["task"]>()
            .mockResolvedValue(createTaskSnapshot({ state: "succeeded" }));
        renderPage(
            api({
                querySources: vi.fn().mockResolvedValue(createSourcePage()),
                syncHealth: vi
                    .fn()
                    .mockResolvedValue(createSyncHealthSummary()),
                startSync,
                task,
            }),
        );

        await user.click(
            await screen.findByRole("button", { name: "同步全部 RSS/Atom" }),
        );

        await waitFor(() => expect(startSync).toHaveBeenCalledTimes(1));
        expect(startSync.mock.calls[0][0]).toMatchObject({
            contract_version: 1,
            target: { kind: "all_enabled_rss_atom" },
            foreground_budget_ms: 30_000,
        });
        expect(startSync.mock.calls[0][0].idempotency_key).toMatch(
            /^rss-sync-/,
        );
        expect(await screen.findByTestId("latest-sync-task")).toHaveTextContent(
            "同步成功",
        );
        expect(task).toHaveBeenCalledWith("task:0123456789abcdef01234567");
    });

    test("starts only the selected enabled RSS source", async () => {
        const user = userEvent.setup();
        const source = createSourceView();
        const startSync = vi
            .fn<DesktopApi["startSync"]>()
            .mockResolvedValue(createTaskRef());
        renderPage(
            api({
                querySources: vi
                    .fn()
                    .mockResolvedValue(createSourcePage({ items: [source] })),
                syncHealth: vi
                    .fn()
                    .mockResolvedValue(createSyncHealthSummary()),
                startSync,
            }),
        );

        await user.click(
            await screen.findByRole("button", { name: "立即同步" }),
        );

        await waitFor(() => expect(startSync).toHaveBeenCalledTimes(1));
        expect(startSync.mock.calls[0][0].target).toEqual({
            kind: "source_id",
            source_id: source.source_id,
        });
    });

    test("keeps partial failure local to its RSS source while preserving successful sources", async () => {
        const failedSource = createSourceView({
            source_id: "source:aaaaaaaaaaaaaaaaaaaaaaaa",
            display_url: "https://example.com/failed.xml",
            status: "error",
        });
        const successfulSource = createSourceView({
            source_id: "source:bbbbbbbbbbbbbbbbbbbbbbbb",
            display_url: "https://example.com/success.xml",
        });
        const failedResult = createSourceSyncStatus({
            source_id: failedSource.source_id,
            state: "failed",
            last_success_at: null,
            error_code: "source_format.rss_atom",
        });
        const successfulResult = createSourceSyncStatus({
            source_id: successfulSource.source_id,
        });
        const partialTask = createTaskSnapshot({
            state: "partially_succeeded",
            sources: [successfulResult, failedResult],
            error_summary: "source_format.rss_atom",
        });
        renderPage(
            api({
                querySources: vi.fn().mockResolvedValue(
                    createSourcePage({
                        items: [successfulSource, failedSource],
                    }),
                ),
                syncHealth: vi.fn().mockResolvedValue(
                    createSyncHealthSummary({
                        latest_task: partialTask,
                        source_results: [successfulResult, failedResult],
                        readiness: createSourceDeliveryReadiness({
                            status: "blocked",
                            sources: [
                                createSourceReadiness({
                                    source_id: successfulSource.source_id,
                                }),
                                createSourceReadiness({
                                    source_id: failedSource.source_id,
                                    status: "failed",
                                    last_success_at: null,
                                }),
                            ],
                        }),
                    }),
                ),
            }),
        );

        expect(
            await screen.findByText(
                /部分来源同步成功，其他 RSS\/Atom 来源仍需处理/,
            ),
        ).toBeVisible();
        expect(screen.getByText(successfulSource.display_url)).toBeVisible();
        expect(screen.getByText(failedSource.display_url)).toBeVisible();
        expect(
            screen.getByText("来源错误：source_format.rss_atom"),
        ).toBeVisible();
    });

    test("disables a source before its retry deadline and explains why", async () => {
        vi.spyOn(Date, "now").mockReturnValue(
            Date.parse("2026-08-18T02:00:00.000Z"),
        );
        const retryAt = "2099-08-18T03:00:00.000Z";
        const source = createSourceView({
            status: "retry_wait",
            retryability: "after",
            next_allowed_at: retryAt,
        });
        renderPage(
            api({
                querySources: vi
                    .fn()
                    .mockResolvedValue(createSourcePage({ items: [source] })),
                syncHealth: vi.fn().mockResolvedValue(
                    createSyncHealthSummary({
                        readiness: createSourceDeliveryReadiness({
                            status: "blocked",
                            sources: [
                                createSourceReadiness({
                                    status: "retry_wait",
                                    next_allowed_at: retryAt,
                                }),
                            ],
                        }),
                        source_results: [
                            createSourceSyncStatus({
                                state: "retry_wait",
                                next_allowed_at: retryAt,
                                error_code: "rate_limit.source",
                            }),
                        ],
                    }),
                ),
            }),
        );

        expect(
            await screen.findByRole("button", { name: "立即同步" }),
        ).toBeDisabled();
        expect(screen.getByText(`服务端要求等待至 ${retryAt}。`)).toBeVisible();
        expect(document.body).not.toHaveTextContent(
            /GitHub|arXiv|三来源已就绪/,
        );
    });

    test("does not let a late lower task revision replace newer health state", async () => {
        const user = userEvent.setup();
        const authoritative = createTaskSnapshot({
            state: "succeeded",
            revision: 5,
        });
        const late = createTaskSnapshot({
            state: "running",
            revision: 2,
            finished_at: null,
        });
        renderPage(
            api({
                querySources: vi.fn().mockResolvedValue(createSourcePage()),
                startSync: vi.fn().mockResolvedValue(createTaskRef()),
                task: vi.fn().mockResolvedValue(late),
                syncHealth: vi
                    .fn()
                    .mockResolvedValue(
                        createSyncHealthSummary({ latest_task: authoritative }),
                    ),
            }),
        );

        await user.click(
            await screen.findByRole("button", { name: "同步全部 RSS/Atom" }),
        );

        await waitFor(() =>
            expect(screen.getByTestId("latest-sync-task")).toHaveTextContent(
                "同步成功（修订版5）",
            ),
        );
    });

    test("re-enables manual source sync after the retry deadline has passed", async () => {
        vi.spyOn(Date, "now").mockReturnValue(
            Date.parse("2026-08-18T02:00:00.000Z"),
        );
        const retryAt = "2020-01-01T00:00:00.000Z";
        renderPage(
            api({
                querySources: vi.fn().mockResolvedValue(
                    createSourcePage({
                        items: [
                            createSourceView({
                                status: "retry_wait",
                                retryability: "after",
                                next_allowed_at: retryAt,
                            }),
                        ],
                    }),
                ),
                syncHealth: vi.fn().mockResolvedValue(
                    createSyncHealthSummary({
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
                ),
            }),
        );

        expect(
            await screen.findByRole("button", { name: "立即同步" }),
        ).toBeEnabled();
        expect(screen.getByText("只同步此 RSS/Atom 来源。")).toBeVisible();
    });

    test("reuses the same sync intent key after a start timeout", async () => {
        const user = userEvent.setup();
        const timeout = Object.assign(new Error("redacted"), {
            code: "timeout.desktop_command",
        });
        const startSync = vi
            .fn<DesktopApi["startSync"]>()
            .mockRejectedValueOnce(timeout)
            .mockResolvedValueOnce(createTaskRef());
        renderPage(
            api({
                querySources: vi.fn().mockResolvedValue(createSourcePage()),
                syncHealth: vi
                    .fn()
                    .mockResolvedValue(createSyncHealthSummary()),
                startSync,
            }),
        );
        const button = await screen.findByRole("button", {
            name: "同步全部 RSS/Atom",
        });

        await user.click(button);
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "再次尝试会复用同一同步意图",
        );
        await user.click(button);

        await waitFor(() => expect(startSync).toHaveBeenCalledTimes(2));
        expect(startSync.mock.calls[0][0].idempotency_key).toBe(
            startSync.mock.calls[1][0].idempotency_key,
        );
    });

    test("uses bounded adaptive polling for retry_wait and stops at its deadline", () => {
        const now = Date.parse("2026-08-18T02:00:00.000Z");
        const retryTask = createTaskSnapshot({
            state: "retry_wait",
            finished_at: null,
            sources: [
                createSourceSyncStatus({
                    state: "retry_wait",
                    next_allowed_at: "2026-08-18T02:02:00.000Z",
                    error_code: "rate_limit.source",
                }),
            ],
        });

        expect(taskPollInterval(retryTask, 1_500, now)).toBe(30_000);
        expect(taskPollInterval(retryTask, 1_500, now + 117_500)).toBe(2_500);
        expect(taskPollInterval(retryTask, 1_500, now + 120_000)).toBe(false);
        expect(
            taskPollInterval(
                createTaskSnapshot({ state: "running", finished_at: null }),
                1_500,
                now,
            ),
        ).toBe(1_500);
        expect(taskPollInterval(createTaskSnapshot(), 1_500, now)).toBe(false);
    });

    test("keeps polling and blocks duplicate controls for every active source", async () => {
        const firstSource = createSourceView({
            source_id: "source:111111111111111111111111",
            display_url: "https://example.com/one.xml",
        });
        const secondSource = createSourceView({
            source_id: "source:222222222222222222222222",
            display_url: "https://example.com/two.xml",
        });
        const firstStatus = createSourceSyncStatus({
            source_id: firstSource.source_id,
            state: "running",
            last_success_at: null,
        });
        const secondStatus = createSourceSyncStatus({
            source_id: secondSource.source_id,
            state: "queued",
            last_success_at: null,
        });
        const latest = createTaskSnapshot({
            state: "running",
            finished_at: null,
            error_summary: null,
            sources: [firstStatus],
        });
        const health = createSyncHealthSummary({
            latest_task: latest,
            pending_task_count: 2,
            source_results: [firstStatus, secondStatus],
            readiness: createSourceDeliveryReadiness({
                status: "syncing",
                sources: [
                    createSourceReadiness({
                        source_id: firstSource.source_id,
                        status: "syncing",
                        last_success_at: null,
                    }),
                    createSourceReadiness({
                        source_id: secondSource.source_id,
                        status: "syncing",
                        last_success_at: null,
                    }),
                ],
            }),
        });
        renderPage(
            api({
                querySources: vi.fn().mockResolvedValue(
                    createSourcePage({
                        items: [firstSource, secondSource],
                    }),
                ),
                syncHealth: vi.fn().mockResolvedValue(health),
                task: vi.fn().mockResolvedValue(latest),
            }),
        );

        expect(
            await screen.findByRole("button", { name: "同步全部 RSS/Atom" }),
        ).toBeDisabled();
        expect(await screen.findByText(firstSource.display_url)).toBeVisible();
        expect(await screen.findByText(secondSource.display_url)).toBeVisible();
        for (const button of screen.getAllByRole("button", {
            name: "立即同步",
        })) {
            expect(button).toBeDisabled();
        }
        expect(healthPollInterval(health, 3_000)).toBe(3_000);
    });

    test("sync mutation preserves focused control and document scroll position", async () => {
        const user = userEvent.setup();
        let resolveStart:
            ((value: ReturnType<typeof createTaskRef>) => void) | undefined;
        const startSync = vi.fn<DesktopApi["startSync"]>().mockReturnValue(
            new Promise((resolve) => {
                resolveStart = resolve;
            }),
        );
        renderPage(
            api({
                querySources: vi.fn().mockResolvedValue(createSourcePage()),
                startSync,
            }),
        );
        const button = await screen.findByRole("button", {
            name: "同步全部 RSS/Atom",
        });
        document.documentElement.scrollTop = 160;
        button.focus();
        await user.click(button);
        expect(button).toHaveFocus();
        expect(document.documentElement.scrollTop).toBe(160);
        resolveStart?.(createTaskRef());
        await waitFor(() => expect(startSync).toHaveBeenCalledTimes(1));
        expect(await screen.findByTestId("latest-sync-task")).toHaveTextContent(
            "同步成功",
        );
        expect(document.documentElement.scrollTop).toBe(160);
    });

    test("terminal task with a run reference exposes the exact result route", async () => {
        const latest = createTaskSnapshot({
            state: "succeeded",
            result_ref: "run:fedcba9876543210fedcba98",
        });
        renderPage(
            api({
                syncHealth: vi.fn().mockResolvedValue(
                    createSyncHealthSummary({
                        latest_task: latest,
                        pending_task_count: 0,
                    }),
                ),
            }),
        );

        expect(
            await screen.findByRole("link", { name: "查看本轮结果" }),
        ).toHaveAttribute("href", "/sync/run:fedcba9876543210fedcba98");
    });

    test("legacy null and active tasks never expose a synthetic result route", async () => {
        const latest = createTaskSnapshot({
            state: "succeeded",
            result_ref: null,
        });
        renderPage(
            api({
                syncHealth: vi.fn().mockResolvedValue(
                    createSyncHealthSummary({
                        latest_task: latest,
                        pending_task_count: 0,
                    }),
                ),
            }),
        );

        expect(await screen.findByTestId("latest-sync-task")).toBeVisible();
        expect(
            screen.queryByRole("link", { name: "查看本轮结果" }),
        ).not.toBeInTheDocument();
    });
});
