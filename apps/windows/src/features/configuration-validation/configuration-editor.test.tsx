import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";

import { DesktopApiProvider } from "../../app/providers/desktop-api-provider";
import type {
    AttentionConfigurationV1,
    ConfigurationValidationResultV1,
    ConfigurationViewV1,
    DesktopApi,
} from "../../lib/desktop-api/desktop-api";
import { DesktopCommandError } from "../../lib/desktop-api/desktop-api";
import { createAppQueryClient } from "../../lib/query-client";
import { ConfigurationEditor } from "./configuration-editor";
import blockingCases from "../../../../../contracts/fixtures/configuration-validation/blocking/cases-v1.json";
import narrowingCases from "../../../../../contracts/fixtures/configuration-validation/narrowing/cases-v1.json";

const configuration: AttentionConfigurationV1 = {
    contract_version: 1,
    tracks: [{ id: "rust", name: "Rust", enabled: true }],
    include_expression: "",
    exclude_expression: "",
    source_preferences: [
        {
            source_kind: "rss",
            identifier: "https://example.com/feed.xml",
            enabled: true,
            trust: 90,
        },
    ],
    refresh_enabled: true,
    refresh_interval_minutes: 60,
    minimum_trust: 0,
    maximum_trust: 100,
    alert_threshold: 80,
    quiet_hours: { enabled: false, start: "22:00", end: "07:00" },
    notification_frequency: { enabled: false, max_per_24h: null },
    active_from: null,
    active_until: null,
};

const current: ConfigurationViewV1 = {
    contract_version: 1,
    revision: 1,
    validator_version: "attention-configuration-v1",
    normalized_config_hash: "a".repeat(64),
    configuration,
    updated_at_ms: 1_700_000_000_000,
};

function result(
    overrides: Partial<ConfigurationValidationResultV1> = {},
): ConfigurationValidationResultV1 {
    return {
        contract_version: 1,
        blocking_errors: [],
        narrowing_risks: [],
        validator_version: "attention-configuration-v1",
        normalized_config_hash: "b".repeat(64),
        validation_receipt: null,
        ...overrides,
    };
}

function api(validation: ConfigurationValidationResultV1) {
    const saveConfiguration = vi
        .fn<DesktopApi["saveConfiguration"]>()
        .mockResolvedValue({
            ...current,
            revision: 2,
            normalized_config_hash: validation.normalized_config_hash,
        });
    const validateConfiguration = vi
        .fn<DesktopApi["validateConfiguration"]>()
        .mockResolvedValue(validation);
    const configurationQuery = vi
        .fn<DesktopApi["configuration"]>()
        .mockResolvedValue(current);
    const desktopApi: DesktopApi = {
        health: vi.fn(),
        demoBootstrap: vi.fn(),
        demoSearch: vi.fn(),
        demoList: vi.fn(),
        demoFilter: vi.fn(),
        demoDetail: vi.fn(),
        setupProgress: vi.fn(),
        saveSetupStep: vi.fn(),
        configuration: configurationQuery,
        validateConfiguration,
        saveConfiguration,
        saveSource: vi.fn(),
        querySources: vi.fn(),
        startSync: vi.fn(),
        task: vi.fn(),
        syncHealth: vi.fn(),
        getSyncResult: vi.fn(),
        queryIntelFeed: vi.fn(),
        queryIntelEvidenceDetail: vi.fn(),
        openIntelOriginal: vi.fn(),
    };
    return {
        desktopApi,
        configurationQuery,
        saveConfiguration,
        validateConfiguration,
    };
}

function renderEditor(desktopApi: DesktopApi) {
    render(
        <QueryClientProvider client={createAppQueryClient()}>
            <DesktopApiProvider api={desktopApi}>
                <MemoryRouter>
                    <ConfigurationEditor />
                </MemoryRouter>
            </DesktopApiProvider>
        </QueryClientProvider>,
    );
}

describe("configuration editor", () => {
    it("renders a recoverable error when the initial configuration query fails", async () => {
        const harness = api(result());
        harness.configurationQuery.mockRejectedValue(
            new Error("controlled failure"),
        );
        renderEditor(harness.desktopApi);

        expect(await screen.findByRole("alert")).toHaveTextContent(
            "配置暂时不可用",
        );
        expect(
            screen.queryByText("正在读取当前设备配置…"),
        ).not.toBeInTheDocument();
    });

    it("saves a valid configuration directly without a risk confirmation", async () => {
        const harness = api(result());
        renderEditor(harness.desktopApi);

        await userEvent.click(
            await screen.findByRole("button", { name: "保存配置" }),
        );

        await waitFor(() =>
            expect(harness.saveConfiguration).toHaveBeenCalledTimes(1),
        );
        expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
        expect(
            await screen.findByText(/已保存，将在下一轮同步生效/),
        ).toBeVisible();
    });

    it("blocks saving and focuses the first invalid field", async () => {
        const sharedCase = blockingCases.cases[0];
        const harness = api(
            result({
                blocking_errors: [
                    {
                        field_path: sharedCase.field_path,
                        code: sharedCase.expected_code as "expression_unparseable",
                        message_key: "configuration.fix.value_out_of_range",
                    },
                ],
            }),
        );
        renderEditor(harness.desktopApi);

        await userEvent.click(
            await screen.findByRole("button", { name: "保存配置" }),
        );

        expect(await screen.findByRole("alert")).toBeVisible();
        await waitFor(() =>
            expect(screen.getByLabelText("包含表达式")).toHaveFocus(),
        );
        expect(harness.saveConfiguration).not.toHaveBeenCalled();
    });

    it("renders every blocking error and associates affected fields", async () => {
        const harness = api(
            result({
                blocking_errors: [
                    {
                        field_path: "include_expression",
                        code: "expression_unparseable",
                        message_key: "configuration.fix.expression_unparseable",
                    },
                    {
                        field_path: "minimum_trust",
                        code: "value_out_of_range",
                        message_key: "configuration.fix.value_out_of_range",
                    },
                ],
            }),
        );
        renderEditor(harness.desktopApi);

        await userEvent.click(
            await screen.findByRole("button", { name: "保存配置" }),
        );

        expect(await screen.findAllByRole("listitem")).toHaveLength(2);
        expect(screen.getByLabelText("包含表达式")).toHaveAttribute(
            "aria-describedby",
            "configuration-blocking-errors",
        );
        expect(screen.getByLabelText("最低可信度（0–100）")).toHaveAttribute(
            "aria-invalid",
            "true",
        );
    });

    it("requires explicit confirmation for a narrowing risk", async () => {
        const sharedCase = narrowingCases.cases[0];
        const receipt = {
            token: "A".repeat(43),
            normalized_config_hash: "b".repeat(64),
            validator_version: "attention-configuration-v1" as const,
        };
        const harness = api(
            result({
                narrowing_risks: [
                    {
                        code: sharedCase.expected_code as "all_sources_disabled",
                        condition_key:
                            "configuration.risk.all_sources_disabled.condition",
                        consequence_key:
                            "configuration.risk.all_sources_disabled.consequence",
                    },
                ],
                validation_receipt: receipt,
            }),
        );
        renderEditor(harness.desktopApi);

        await userEvent.click(
            await screen.findByRole("button", { name: "保存配置" }),
        );
        expect(await screen.findByRole("alertdialog")).toBeVisible();
        expect(harness.saveConfiguration).not.toHaveBeenCalled();
        await userEvent.click(
            screen.getByRole("button", { name: "理解风险并保存" }),
        );

        await waitFor(() =>
            expect(harness.saveConfiguration).toHaveBeenCalledTimes(1),
        );
        expect(
            harness.saveConfiguration.mock.calls[0]?.[0].validation_receipt,
        ).toEqual(receipt);
    });

    it("closes the risk dialog with Escape and restores focus to save", async () => {
        const harness = api(
            result({
                narrowing_risks: [
                    {
                        code: "all_high_trust_candidates_filtered",
                        condition_key: "configuration.risk.filtered.condition",
                        consequence_key:
                            "configuration.risk.filtered.consequence",
                    },
                ],
                validation_receipt: {
                    token: "B".repeat(43),
                    normalized_config_hash: "b".repeat(64),
                    validator_version: "attention-configuration-v1",
                },
            }),
        );
        renderEditor(harness.desktopApi);
        const save = await screen.findByRole("button", { name: "保存配置" });
        await userEvent.click(save);

        expect(screen.getByRole("button", { name: "返回修改" })).toHaveFocus();
        await userEvent.keyboard("{Escape}");
        expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
        await waitFor(() => expect(save).toHaveFocus());
    });

    it("preserves the same save intent when a failed save is retried", async () => {
        const harness = api(result());
        harness.saveConfiguration
            .mockRejectedValueOnce(new Error("controlled failure"))
            .mockResolvedValueOnce({
                ...current,
                revision: 2,
                normalized_config_hash: "b".repeat(64),
            });
        renderEditor(harness.desktopApi);
        const save = await screen.findByRole("button", { name: "保存配置" });

        await userEvent.click(save);
        expect(await screen.findByText("状态：save_error")).toBeVisible();
        await userEvent.click(save);
        await waitFor(() =>
            expect(harness.saveConfiguration).toHaveBeenCalledTimes(2),
        );
        expect(
            harness.saveConfiguration.mock.calls[1]?.[0].idempotency_key,
        ).toBe(harness.saveConfiguration.mock.calls[0]?.[0].idempotency_key);
    });

    it("revalidates instead of replaying a stale validation receipt", async () => {
        const harness = api(result());
        harness.saveConfiguration.mockRejectedValueOnce(
            new DesktopCommandError({
                contract_version: 1,
                code: "validation.stale_validation_receipt",
                category: "validation",
                message_key: "configuration.stale_validation_receipt",
                retryability: "after_user_action",
                source_id: null,
                task_id: null,
                details_allowlisted: "",
                correlation_id: "corr_stale",
            }),
        );
        renderEditor(harness.desktopApi);
        const saveButton = await screen.findByRole("button", {
            name: "保存配置",
        });

        await userEvent.click(saveButton);
        expect(await screen.findByText("状态：save_error")).toBeVisible();
        await userEvent.click(saveButton);

        await waitFor(() =>
            expect(harness.validateConfiguration).toHaveBeenCalledTimes(2),
        );
    });

    it("keeps save disabled while validation is in flight", async () => {
        const harness = api(result());
        let finishValidation!: (value: ConfigurationValidationResultV1) => void;
        harness.validateConfiguration.mockImplementation(
            () =>
                new Promise((resolve) => {
                    finishValidation = resolve;
                }),
        );
        renderEditor(harness.desktopApi);
        const saveButton = await screen.findByRole("button", {
            name: "保存配置",
        });

        await userEvent.click(saveButton);
        expect(
            screen.getByRole("button", { name: "正在校验…" }),
        ).toBeDisabled();
        finishValidation(result());

        await waitFor(() =>
            expect(harness.saveConfiguration).toHaveBeenCalledOnce(),
        );
    });

    it("preserves an empty numeric draft until core validation reports it", async () => {
        const harness = api(
            result({
                blocking_errors: [
                    {
                        field_path: "refresh_interval_minutes",
                        code: "value_out_of_range",
                        message_key: "configuration.fix.value_out_of_range",
                    },
                ],
            }),
        );
        renderEditor(harness.desktopApi);
        const refresh = await screen.findByLabelText("刷新周期（分钟）");

        await userEvent.clear(refresh);
        expect(refresh).toHaveValue(null);
        await userEvent.click(screen.getByRole("button", { name: "保存配置" }));

        expect(await screen.findByText(/刷新周期：/)).toBeVisible();
        expect(
            harness.validateConfiguration.mock.calls[0]?.[0].configuration
                .refresh_interval_minutes,
        ).toBe(0);
        expect(refresh).toHaveValue(null);
    });

    it("protects a dirty draft from application-link navigation", async () => {
        const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
        const harness = api(result());
        renderEditor(harness.desktopApi);
        const trackName = await screen.findByLabelText("赛道名称 1");
        await userEvent.clear(trackName);
        await userEvent.type(trackName, "Rust 新赛道");

        await userEvent.click(screen.getByRole("link", { name: "返回设置" }));

        expect(confirm).toHaveBeenCalledOnce();
        expect(screen.getByDisplayValue("Rust 新赛道")).toBeVisible();
        expect(screen.getByText("状态：dirty")).toBeVisible();
    });

    it("supports adding, disabling, and deleting track draft rows", async () => {
        const harness = api(result());
        renderEditor(harness.desktopApi);

        await userEvent.click(
            await screen.findByRole("button", { name: "添加赛道" }),
        );
        expect(screen.getByLabelText("赛道名称 2")).toHaveValue("新赛道");
        await userEvent.click(screen.getAllByLabelText("启用此赛道")[1]);
        expect(screen.getAllByLabelText("启用此赛道")[1]).not.toBeChecked();
        await userEvent.click(
            screen.getAllByRole("button", { name: "删除赛道" })[1],
        );
        expect(screen.queryByLabelText("赛道名称 2")).not.toBeInTheDocument();
    });

    it("supports adding, editing, disabling, and deleting source preferences", async () => {
        const harness = api(result());
        renderEditor(harness.desktopApi);

        await userEvent.click(
            await screen.findByRole("button", { name: "添加来源" }),
        );
        const identifiers = screen.getAllByLabelText("来源地址或标识");
        expect(identifiers).toHaveLength(2);
        await userEvent.clear(identifiers[1]);
        await userEvent.type(identifiers[1], "https://example.com/feed.xml");
        await userEvent.click(screen.getAllByLabelText("启用此来源")[1]);
        expect(screen.getAllByLabelText("启用此来源")[1]).not.toBeChecked();
        await userEvent.click(
            screen.getAllByRole("button", { name: "删除来源" })[1],
        );
        expect(screen.getAllByLabelText("来源地址或标识")).toHaveLength(1);
    });
});
