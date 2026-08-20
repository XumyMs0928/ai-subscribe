import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router";
import { describe, expect, it, vi } from "vitest";

import { DesktopApiProvider } from "../../app/providers/desktop-api-provider";
import type {
    DesktopApi,
    SaveSetupStepInputV1,
    SetupProgressV1,
} from "../../lib/desktop-api/desktop-api";
import {
    createAppQueryClient,
    desktopApiQueryKey,
    setupKeys,
} from "../../lib/query-client";
import { SettingsRoot } from "../settings/settings-root";
import { ProgressiveSetupGuide } from "./progressive-setup-guide";
import { SetupGuideEntry } from "./setup-guide-entry";

function progress(overrides: Partial<SetupProgressV1> = {}): SetupProgressV1 {
    return {
        contract_version: 1,
        revision: 0,
        configuration_revision: 1,
        overall_status: "not_started",
        steps: [
            {
                contract_version: 1,
                step_id: "tracks",
                status: "not_started",
                saved_fields_version: null,
            },
            {
                contract_version: 1,
                step_id: "source_examples",
                status: "not_started",
                saved_fields_version: null,
            },
            {
                contract_version: 1,
                step_id: "refresh_cadence",
                status: "not_started",
                saved_fields_version: null,
            },
            {
                contract_version: 1,
                step_id: "ai_data_disclosure",
                status: "not_started",
                saved_fields_version: null,
            },
        ],
        next_step_id: "tracks",
        defaults: {
            contract_version: 1,
            fixture_id: "setup-defaults-v1",
            default_track_ids: ["ai_agents"],
            default_source_example_ids: ["github_releases"],
            default_refresh_cadence: "manual",
            tracks: [{ id: "ai_agents", label: "AI 智能体", is_demo: true }],
            source_examples: [
                {
                    id: "github_releases",
                    label: "GitHub Release 示例",
                    is_demo: true,
                },
            ],
            refresh_cadences: [
                { id: "manual", label: "仅手动刷新", is_demo: false },
            ],
        },
        saved_config: {
            track_ids: ["ai_agents"],
            source_example_ids: ["github_releases"],
            refresh_cadence: "manual",
            ai_data_disclosure_acknowledged: false,
        },
        ...overrides,
    };
}

function api(overrides: Partial<DesktopApi> = {}): DesktopApi {
    return {
        health: vi.fn(),
        demoBootstrap: vi.fn(),
        demoSearch: vi.fn(),
        demoList: vi.fn(),
        demoFilter: vi.fn(),
        demoDetail: vi.fn(),
        setupProgress: vi.fn().mockResolvedValue(progress()),
        saveSetupStep: vi.fn(),
        configuration: vi.fn(),
        validateConfiguration: vi.fn(),
        saveConfiguration: vi.fn(),
        saveSource: vi.fn(),
        querySources: vi.fn(),
        startSync: vi.fn(),
        task: vi.fn(),
        syncHealth: vi.fn(),
        getSyncResult: vi.fn(),
        queryIntelFeed: vi.fn(),
        ...overrides,
    };
}

function renderRoutes(desktopApi: DesktopApi, initial = "/settings/setup") {
    const queryClient = createAppQueryClient();
    const rendered = render(
        <QueryClientProvider client={queryClient}>
            <DesktopApiProvider api={desktopApi}>
                <MemoryRouter initialEntries={[initial]}>
                    <Routes>
                        <Route path="/settings" element={<SettingsRoot />} />
                        <Route
                            path="/settings/setup"
                            element={<ProgressiveSetupGuide />}
                        />
                        <Route path="/" element={<p>主情报流</p>} />
                    </Routes>
                </MemoryRouter>
            </DesktopApiProvider>
        </QueryClientProvider>,
    );
    return { ...rendered, queryClient };
}

describe("progressive setup guide", () => {
    it.each([1440, 1100, 800, 560])(
        "keeps the guide controls operable at the %dpx layout class",
        async (width) => {
            Object.defineProperty(window, "innerWidth", {
                configurable: true,
                value: width,
            });
            window.dispatchEvent(new Event("resize"));
            renderRoutes(api());
            expect(
                await screen.findByRole("heading", { name: "配置引导" }),
            ).toBeVisible();
            expect(
                await screen.findByRole("button", { name: "稍后继续" }),
            ).toBeEnabled();
        },
    );

    it("shows a non-blocking loading state while progress is pending", () => {
        renderRoutes(
            api({
                setupProgress: vi.fn(() => new Promise<never>(() => undefined)),
            }),
        );
        expect(screen.getByRole("status")).toHaveTextContent(
            "正在恢复配置进度",
        );
        expect(screen.getByText("仅影响此 Windows 设备")).toBeVisible();
    });

    it.each([
        ["not_started", "未开始"],
        ["in_progress", "进行中"],
        ["skipped", "已跳过"],
        ["partially_completed", "部分完成"],
        ["completed", "已完成"],
    ] as const)(
        "renders the %s state with a non-color label",
        (status, label) => {
            render(
                <MemoryRouter>
                    <SetupGuideEntry status={status} />
                </MemoryRouter>,
            );
            expect(
                screen.getByRole("link", { name: `配置引导，${label}` }),
            ).toBeVisible();
        },
    );

    it("shows a stable settings entry and device-only scope", async () => {
        renderRoutes(api(), "/settings");
        expect(
            await screen.findByRole("link", { name: "配置引导，未开始" }),
        ).toHaveAttribute("href", "/settings/setup");
        expect(screen.getByText("仅影响此 Windows 设备")).toBeVisible();
        expect(screen.getByText(/不提供云备份或跨设备同步/)).toBeVisible();
    });

    it("keeps the draft and current step when saving fails", async () => {
        const saveSetupStep = vi
            .fn()
            .mockRejectedValue(new Error("storage unavailable"));
        renderRoutes(api({ saveSetupStep }));
        const user = userEvent.setup();
        const option = await screen.findByRole("checkbox", {
            name: /AI 智能体/,
        });
        expect(option).toBeChecked();
        await user.click(screen.getByRole("button", { name: "保存并继续" }));
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "当前选择仍保留",
        );
        expect(option).toBeChecked();
        expect(saveSetupStep).toHaveBeenCalledWith(
            expect.objectContaining({
                step_id: "tracks",
                action: "save",
                selected_values: ["ai_agents"],
                expected_revision: 0,
            }),
        );
    });

    it("returns to the feed after core confirms skip", async () => {
        const skipped = progress({
            revision: 1,
            overall_status: "skipped",
            steps: progress().steps.map((item) =>
                item.step_id === "tracks"
                    ? { ...item, status: "skipped" }
                    : item,
            ),
            next_step_id: "tracks",
        });
        const saveSetupStep = vi.fn().mockResolvedValue(skipped);
        renderRoutes(api({ saveSetupStep }));
        await userEvent.click(
            await screen.findByRole("button", { name: "跳过此步" }),
        );
        expect(await screen.findByText("主情报流")).toBeVisible();
    });

    it("later returns to the feed only after core confirms the new revision", async () => {
        const next = progress({ revision: 1, overall_status: "in_progress" });
        const saveSetupStep = vi.fn().mockResolvedValue(next);
        renderRoutes(api({ saveSetupStep }));
        await userEvent.click(
            await screen.findByRole("button", { name: "稍后继续" }),
        );
        expect(await screen.findByText("主情报流")).toBeVisible();
        expect(saveSetupStep).toHaveBeenCalledWith(
            expect.objectContaining({ action: "later", selected_values: [] }),
        );
    });

    it("recovers a progress read error through the visible retry", async () => {
        const setupProgress = vi
            .fn()
            .mockRejectedValueOnce(new Error("read failed"))
            .mockResolvedValueOnce(progress());
        renderRoutes(api({ setupProgress }), "/settings");
        const user = userEvent.setup();
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "配置状态暂时不可用",
        );
        expect(
            screen.getByRole("link", { name: "配置引导，状态暂时不可用" }),
        ).toBeVisible();
        await user.click(screen.getByRole("button", { name: "重试" }));
        expect(
            await screen.findByRole("link", { name: "配置引导，未开始" }),
        ).toBeVisible();
        expect(setupProgress).toHaveBeenCalledTimes(2);
    });

    it("reuses the same idempotency key when an unconfirmed save is retried", async () => {
        const next = progress({
            revision: 1,
            overall_status: "partially_completed",
            steps: progress().steps.map((item) =>
                item.step_id === "tracks"
                    ? {
                          ...item,
                          status: "completed",
                          saved_fields_version: 1,
                      }
                    : item,
            ),
            next_step_id: "source_examples",
        });
        const saveSetupStep = vi
            .fn()
            .mockRejectedValueOnce(new Error("response lost"))
            .mockResolvedValueOnce(next);
        renderRoutes(api({ saveSetupStep }));
        const user = userEvent.setup();
        const saveButton = await screen.findByRole("button", {
            name: "保存并继续",
        });
        await user.click(saveButton);
        await screen.findByRole("alert");
        await user.click(saveButton);
        await screen.findByRole("group", { name: "查看来源示例" });
        expect(saveSetupStep).toHaveBeenCalledTimes(2);
        const first = saveSetupStep.mock.calls[0][0] as SaveSetupStepInputV1;
        const second = saveSetupStep.mock.calls[1][0] as SaveSetupStepInputV1;
        expect(first.idempotency_key).toBe(second.idempotency_key);
    });

    it("uses a collision-resistant idempotency key after remounting", async () => {
        const saveSetupStep = vi
            .fn<DesktopApi["saveSetupStep"]>()
            .mockRejectedValue(new Error("controlled failure"));
        const desktopApi = api({ saveSetupStep });
        const firstRender = renderRoutes(desktopApi);
        await userEvent.click(
            await screen.findByRole("button", { name: "保存并继续" }),
        );
        await screen.findByRole("alert");
        const first = saveSetupStep.mock.calls[0][0];
        firstRender.unmount();

        renderRoutes(desktopApi);
        await userEvent.click(
            await screen.findByRole("button", { name: "保存并继续" }),
        );
        await screen.findByRole("alert");
        const second = saveSetupStep.mock.calls[1][0];

        expect(first.idempotency_key).not.toBe(second.idempotency_key);
        expect(first.idempotency_key).toMatch(/^setup:[0-9a-f-]{36}$/);
        expect(second.idempotency_key).toMatch(/^setup:[0-9a-f-]{36}$/);
    });

    it("confirms before abandoning a dirty setup draft", async () => {
        const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
        renderRoutes(api());
        const option = await screen.findByRole("checkbox", {
            name: /AI 智能体/,
        });
        await userEvent.click(option);
        await userEvent.click(screen.getByRole("link", { name: "返回设置" }));

        expect(confirm).toHaveBeenCalledOnce();
        expect(screen.getByRole("heading", { name: "配置引导" })).toBeVisible();
        expect(option).not.toBeChecked();
    });

    it("registers beforeunload protection for a dirty draft", async () => {
        renderRoutes(api());
        await userEvent.click(
            await screen.findByRole("checkbox", { name: /AI 智能体/ }),
        );
        const event = new Event("beforeunload", { cancelable: true });
        fireEvent(window, event);
        expect(event.defaultPrevented).toBe(true);
    });

    it("blocks links while saving and ignores navigation from a late result after unmount", async () => {
        let resolveSave: ((value: SetupProgressV1) => void) | undefined;
        const saveSetupStep = vi.fn(
            () =>
                new Promise<SetupProgressV1>((resolve) => {
                    resolveSave = resolve;
                }),
        );
        const rendered = renderRoutes(api({ saveSetupStep }));
        await userEvent.click(
            await screen.findByRole("button", { name: "稍后继续" }),
        );
        await userEvent.click(screen.getByRole("link", { name: "返回设置" }));
        expect(screen.getByRole("heading", { name: "配置引导" })).toBeVisible();

        rendered.unmount();
        resolveSave?.(progress({ revision: 1 }));
        await Promise.resolve();
        expect(saveSetupStep).toHaveBeenCalledTimes(1);
    });

    it("does not let a late progress read overwrite a confirmed save", async () => {
        let resolveLate: ((value: SetupProgressV1) => void) | undefined;
        const setupProgress = vi
            .fn()
            .mockResolvedValueOnce(progress())
            .mockImplementationOnce(
                () =>
                    new Promise<SetupProgressV1>((resolve) => {
                        resolveLate = resolve;
                    }),
            );
        const next = progress({
            revision: 1,
            overall_status: "partially_completed",
            steps: progress().steps.map((item) =>
                item.step_id === "tracks"
                    ? {
                          ...item,
                          status: "completed",
                          saved_fields_version: 1,
                      }
                    : item,
            ),
            next_step_id: "source_examples",
        });
        const desktopApi = api({
            setupProgress,
            saveSetupStep: vi.fn().mockResolvedValue(next),
        });
        const { queryClient } = renderRoutes(desktopApi);
        await screen.findByRole("group", { name: "选择关注赛道" });
        const lateRead = queryClient.refetchQueries({
            queryKey: setupKeys.progress(desktopApiQueryKey(desktopApi)),
        });
        await waitFor(() => expect(setupProgress).toHaveBeenCalledTimes(2));
        await userEvent.click(
            screen.getByRole("button", { name: "保存并继续" }),
        );
        await screen.findByRole("group", { name: "查看来源示例" });
        resolveLate?.(progress());
        await lateRead;
        expect(
            screen.getByRole("group", { name: "查看来源示例" }),
        ).toBeVisible();
    });

    it("restores focus to the fixed setup entry after returning to settings", async () => {
        renderRoutes(api());
        const user = userEvent.setup();
        await user.click(await screen.findByRole("link", { name: "返回设置" }));
        const entry = await screen.findByRole("link", {
            name: "配置引导，未开始",
        });
        await waitFor(() => expect(entry).toHaveFocus());
    });

    it("does not replay a completed guide", async () => {
        const completed = progress({
            revision: 4,
            overall_status: "completed",
            steps: progress().steps.map((step) => ({
                ...step,
                status: "completed",
                saved_fields_version: 1,
            })),
            next_step_id: null,
        });
        renderRoutes(
            api({ setupProgress: vi.fn().mockResolvedValue(completed) }),
        );
        expect(
            await screen.findByRole("heading", { name: "配置引导已完成" }),
        ).toBeVisible();
        expect(
            screen.queryByRole("button", { name: "保存并继续" }),
        ).not.toBeInTheDocument();
    });
});
