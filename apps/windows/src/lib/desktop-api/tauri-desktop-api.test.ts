import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { createTauriDesktopApi } from "./tauri-desktop-api";
import {
    DesktopCommandError,
    isAttentionConfigurationV1,
    isDesktopApiError,
    isSyncHealthSummaryV1,
    isSyncResultPageV1,
    isTaskSnapshotV1,
} from "./desktop-api";
import {
    createSyncHealthSummary,
    createSyncResultPage,
    createTaskSnapshot,
    encodeSyncResultCursor,
} from "../../../../../tests/support/factories/demo-dto.factory";

vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(),
}));

const setupProgressFixture = () =>
    ({
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
                { id: "github_releases", label: "GitHub 示例", is_demo: true },
            ],
            refresh_cadences: [{ id: "manual", label: "手动", is_demo: false }],
        },
        saved_config: {
            track_ids: ["ai_agents"],
            source_example_ids: ["github_releases"],
            refresh_cadence: "manual",
            ai_data_disclosure_acknowledged: false,
        },
    }) as const;

const configurationFixture = () =>
    ({
        contract_version: 1,
        tracks: [{ id: "ai_agents", name: "AI 智能体", enabled: true }],
        include_expression: "AI AND Agent",
        exclude_expression: "deprecated",
        source_preferences: [
            {
                source_kind: "rss",
                identifier: "https://example.invalid/feed.xml",
                enabled: true,
                trust: 90,
            },
        ],
        refresh_enabled: true,
        refresh_interval_minutes: 60,
        minimum_trust: 40,
        maximum_trust: 100,
        alert_threshold: 80,
        quiet_hours: { enabled: false, start: "22:00", end: "07:00" },
        notification_frequency: { enabled: false, max_per_24h: null },
        active_from: null,
        active_until: null,
    }) as const;

const configurationContracts = () => {
    const configuration = configurationFixture();
    const hash =
        "bcd1f22825c821003d536ebee4afe3312740917f54a463d8e1a4fd8b90e8f7fe";
    return {
        configuration,
        hash,
        view: {
            contract_version: 1,
            revision: 1,
            validator_version: "attention-configuration-v1",
            normalized_config_hash: hash,
            configuration,
            updated_at_ms: 1_787_000_000_000,
        } as const,
        validation: {
            contract_version: 1,
            blocking_errors: [],
            narrowing_risks: [],
            validator_version: "attention-configuration-v1",
            normalized_config_hash: hash,
            validation_receipt: null,
        } as const,
    };
};

describe("Tauri DesktopApi transport", () => {
    beforeEach(() => {
        vi.mocked(invoke).mockReset();
    });

    it("uses the only approved release command and preserves contract fields", async () => {
        vi.mocked(invoke).mockResolvedValue({
            contract_version: 1,
            status: "ok",
            checked_at: null,
        });

        const result = await createTauriDesktopApi().health();

        expect(invoke).toHaveBeenCalledOnce();
        expect(invoke).toHaveBeenCalledWith("health_v1");
        expect(result).toEqual({
            contract_version: 1,
            status: "ok",
            checked_at: null,
        });
    });

    it("rejects a malformed response instead of inventing contract defaults", async () => {
        vi.mocked(invoke).mockResolvedValue({
            contract_version: 1,
            status: "ok",
        });

        await expect(createTauriDesktopApi().health()).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
    });

    it("validates and preserves every stable AppError field", () => {
        const wire = {
            contract_version: 1 as const,
            code: "internal.unexpected",
            category: "internal",
            message_key: "error.internal",
            retryability: "manual",
            source_id: null,
            task_id: null,
            details_allowlisted: "",
            correlation_id: "windows-command-1",
        };
        expect(isDesktopApiError(wire)).toBe(true);
        const observed = new DesktopCommandError(wire);
        expect(observed).toMatchObject(wire);
        for (const corrupted of [
            { ...wire, extra: "private" },
            { ...wire, code: "internal.unknown" },
            { ...wire, category: "validation" },
            { ...wire, retryability: "automatic" },
            { ...wire, message_key: "error.validation" },
            { ...wire, task_id: "task:unsafe" },
        ]) {
            expect(isDesktopApiError(corrupted)).toBe(false);
        }
    });

    it("rejects non-UTC and malformed checked_at values", async () => {
        for (const checked_at of [
            "2026-08-14T00:00:00+00:00",
            "2026-02-30T00:00:00Z",
            "not-a-time",
        ]) {
            vi.mocked(invoke).mockResolvedValue({
                contract_version: 1,
                status: "ok",
                checked_at,
            });
            await expect(
                createTauriDesktopApi().health(),
            ).rejects.toMatchObject({
                code: "internal.desktop_contract_mismatch",
            });
        }
    });

    it("uses the exact demo commands and preserves an empty search result", async () => {
        const catalog = {
            contract_version: 1,
            dataset_id: "demo-v1",
            items: [],
        };
        vi.mocked(invoke)
            .mockResolvedValueOnce({
                ...catalog,
                items: [
                    {
                        id: "demo:rust-001",
                        data_origin: "demo",
                        publisher: "Rust Project",
                        title: "Rust update",
                        track: "tools",
                        summary: "Demo summary",
                        original_url: "https://www.rust-lang.org/",
                        importance: "medium",
                        ai_status: "generated",
                        published_at: "2026-06-20T10:00:00Z",
                        collected_at: "2026-06-20T10:30:00Z",
                    },
                ],
            })
            .mockResolvedValueOnce(catalog);
        const api = createTauriDesktopApi();

        await expect(api.demoBootstrap()).resolves.toMatchObject({
            dataset_id: "demo-v1",
        });
        await expect(api.demoSearch("量子", "research")).resolves.toEqual(
            catalog,
        );

        expect(invoke).toHaveBeenNthCalledWith(1, "demo_bootstrap_v1");
        expect(invoke).toHaveBeenNthCalledWith(2, "demo_search_v1", {
            query: "量子",
            track: "research",
        });
    });

    it("rejects an empty bootstrap catalog but accepts it for search", async () => {
        const empty = { contract_version: 1, dataset_id: "demo-v1", items: [] };
        vi.mocked(invoke).mockResolvedValue(empty);
        await expect(
            createTauriDesktopApi().demoBootstrap(),
        ).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
        await expect(
            createTauriDesktopApi().demoSearch("missing", null),
        ).resolves.toEqual(empty);
    });

    it("keeps additive v1 summary compatibility while rejecting detail fields in lists", async () => {
        const legacyV1Item = {
            id: "demo:legacy-v1",
            data_origin: "demo",
            publisher: "Legacy Publisher",
            title: "Legacy v1 summary",
            track: "legacy",
            summary: "Payload produced before evidence fields were added.",
            original_url: "https://example.com/legacy",
            published_at: "2026-01-01T00:00:00Z",
            collected_at: "2026-01-01T00:01:00Z",
        };
        vi.mocked(invoke)
            .mockResolvedValueOnce({
                contract_version: 1,
                dataset_id: "demo-v1",
                items: [legacyV1Item],
            })
            .mockResolvedValueOnce({
                contract_version: 1,
                dataset_id: "demo-v1",
                items: [{ ...legacyV1Item, facts: ["detail-only leak"] }],
            });

        await expect(
            createTauriDesktopApi().demoBootstrap(),
        ).resolves.toMatchObject({
            items: [legacyV1Item],
        });
        await expect(
            createTauriDesktopApi().demoBootstrap(),
        ).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
    });

    it("validates demo detail responses at the transport boundary", async () => {
        vi.mocked(invoke).mockResolvedValue({
            id: "real:forged",
            data_origin: "demo",
        });

        await expect(
            createTauriDesktopApi().demoDetail("demo:missing"),
        ).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
        expect(invoke).toHaveBeenCalledWith("demo_detail_v1", {
            id: "demo:missing",
        });
    });

    it("accepts only complete versioned evidence details", async () => {
        const detail = {
            contract_version: 1,
            dataset_id: "demo-v1",
            id: "demo:rust-001",
            data_origin: "demo",
            publisher: "Rust Project",
            title: "Rust update",
            track: "tools",
            summary: "Demo summary",
            original_url: "https://www.rust-lang.org/",
            importance: "high",
            ai_status: "generated",
            published_at: "2026-06-20T10:00:00Z",
            collected_at: "2026-06-20T10:30:00Z",
            what_happened: "A release changed.",
            why_it_matters: "Diagnostics are clearer.",
            possible_impact: "Debugging may be faster.",
            facts: ["Official demo fact"],
            rule_reasons: ["Matches tools track"],
            ai_content: "Demo AI generated content",
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
        };
        vi.mocked(invoke).mockResolvedValueOnce(detail);
        await expect(
            createTauriDesktopApi().demoDetail("demo:rust-001"),
        ).resolves.toEqual(detail);

        for (const malformed of [
            { ...detail, ai_confidence_percent: 101 },
            { ...detail, facts: [] },
            {
                ...detail,
                provenance: {
                    ...detail.provenance,
                    original_url: "http://insecure.example",
                },
            },
            {
                ...detail,
                provenance: { ...detail.provenance, publisher: "Mismatch" },
            },
            {
                ...detail,
                provenance: {
                    ...detail.provenance,
                    deterministic_association_basis: "",
                },
            },
        ]) {
            vi.mocked(invoke).mockResolvedValueOnce(malformed);
            await expect(
                createTauriDesktopApi().demoDetail("demo:rust-001"),
            ).rejects.toMatchObject({
                code: "internal.desktop_contract_mismatch",
            });
        }

        vi.mocked(invoke).mockResolvedValueOnce({
            ...detail,
            id: "demo:different-id",
        });
        await expect(
            createTauriDesktopApi().demoDetail("demo:rust-001"),
        ).rejects.toMatchObject({ code: "internal.desktop_contract_mismatch" });
    });

    it("uses explicit paged list and filter commands", async () => {
        const page = {
            contract_version: 1,
            dataset_id: "demo-v1",
            items: [],
            next_cursor:
                "v1:64656d6f2d7631:-:323032362d30362d32305431303a30303a30305a:64656d6f3a727573742d303031:0123456789abcdef",
        };
        vi.mocked(invoke).mockResolvedValue(page);
        const api = createTauriDesktopApi();

        await expect(api.demoList(null, 2)).resolves.toEqual(page);
        const cursor = page.next_cursor;
        await expect(api.demoFilter("tools", cursor, 2)).resolves.toEqual(page);
        expect(invoke).toHaveBeenNthCalledWith(1, "demo_list_v1", {
            cursor: null,
            limit: 2,
        });
        expect(invoke).toHaveBeenNthCalledWith(2, "demo_filter_v1", {
            track: "tools",
            cursor,
            limit: 2,
        });
    });

    it("validates setup identity, next-step consistency, and exact save input", async () => {
        const progress = setupProgressFixture();
        vi.mocked(invoke)
            .mockResolvedValueOnce(progress)
            .mockResolvedValueOnce({
                ...progress,
                revision: 1,
                overall_status: "partially_completed",
                steps: progress.steps.map((step) =>
                    step.step_id === "tracks"
                        ? {
                              ...step,
                              status: "completed",
                              saved_fields_version: 1,
                          }
                        : step,
                ),
                next_step_id: "source_examples",
                saved_config: {
                    ...progress.saved_config,
                    track_ids: ["ai_agents"],
                },
            });
        const api = createTauriDesktopApi();
        await expect(api.setupProgress()).resolves.toEqual(progress);
        const input = {
            contract_version: 1,
            step_id: "tracks",
            action: "save",
            selected_values: ["ai_agents"],
            expected_revision: 0,
            expected_configuration_revision: 1,
            idempotency_key: "setup:0:tracks:save:1",
        } as const;
        await expect(api.saveSetupStep(input)).resolves.toMatchObject({
            revision: 1,
            next_step_id: "source_examples",
        });
        expect(invoke).toHaveBeenNthCalledWith(1, "setup_progress_v1");
        expect(invoke).toHaveBeenNthCalledWith(2, "save_setup_step_v1", {
            input,
        });

        for (const malformed of [
            { ...progress, next_step_id: "source_examples" },
            { ...progress, overall_status: "completed" },
            { ...progress, revision: -1 },
            { ...progress, steps: [...progress.steps, progress.steps[0]] },
            {
                ...progress,
                saved_config: {
                    ...progress.saved_config,
                    track_ids: ["unknown_track"],
                },
            },
            {
                ...progress,
                defaults: {
                    ...progress.defaults,
                    source_examples: [
                        {
                            id: "github_releases",
                            label: "GitHub",
                            is_demo: false,
                        },
                    ],
                },
            },
        ]) {
            vi.mocked(invoke).mockResolvedValueOnce(malformed);
            await expect(api.setupProgress()).rejects.toMatchObject({
                code: "internal.desktop_contract_mismatch",
            });
        }

        vi.mocked(invoke).mockResolvedValueOnce({
            ...progress,
            revision: 1,
            overall_status: "partially_completed",
            steps: progress.steps.map((step) =>
                step.step_id === "tracks"
                    ? {
                          ...step,
                          status: "completed",
                          saved_fields_version: 1,
                      }
                    : step,
            ),
            next_step_id: "source_examples",
            saved_config: {
                ...progress.saved_config,
                track_ids: ["ai_agents", "unexpected"],
            },
        });
        await expect(api.saveSetupStep(input)).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
    });

    it("enforces exact configuration commands, hashes, and two-channel validation", async () => {
        const { configuration, hash, view, validation } =
            configurationContracts();
        const saveInput = {
            contract_version: 1,
            configuration,
            expected_revision: 1,
            expected_normalized_config_hash: hash,
            idempotency_key: "configuration-save-1",
            validation_receipt: null,
        } as const;
        vi.mocked(invoke)
            .mockResolvedValueOnce(view)
            .mockResolvedValueOnce(validation)
            .mockResolvedValueOnce({ ...view, revision: 2 });
        const api = createTauriDesktopApi();

        await expect(api.configuration()).resolves.toEqual(view);
        await expect(
            api.validateConfiguration({
                contract_version: 1,
                configuration,
            }),
        ).resolves.toEqual(validation);
        await expect(api.saveConfiguration(saveInput)).resolves.toMatchObject({
            revision: 2,
            normalized_config_hash: hash,
        });
        expect(invoke).toHaveBeenNthCalledWith(1, "configuration_v1");
        expect(invoke).toHaveBeenNthCalledWith(2, "validate_configuration_v1", {
            input: { contract_version: 1, configuration },
        });
        expect(invoke).toHaveBeenNthCalledWith(3, "save_configuration_v1", {
            input: saveInput,
        });
    });

    it("rejects contradictory validation and mismatched configuration hashes", async () => {
        const { configuration, hash, view, validation } =
            configurationContracts();
        const saveInput = {
            contract_version: 1,
            configuration,
            expected_revision: 1,
            expected_normalized_config_hash: hash,
            idempotency_key: "configuration-save-1",
            validation_receipt: null,
        } as const;
        const api = createTauriDesktopApi();

        for (const malformed of [
            { ...validation, extra: true },
            {
                ...validation,
                blocking_errors: [
                    {
                        field_path: "tracks",
                        code: "value_out_of_range",
                        message_key: "configuration.fix.value_out_of_range",
                    },
                    {
                        field_path: "tracks",
                        code: "expression_unparseable",
                        message_key: "configuration.fix.expression_unparseable",
                    },
                ],
            },
            {
                ...validation,
                blocking_errors: [
                    {
                        field_path: "z_field",
                        code: "value_out_of_range",
                        message_key: "configuration.fix.value_out_of_range",
                    },
                    {
                        field_path: "a_field",
                        code: "value_out_of_range",
                        message_key: "configuration.fix.value_out_of_range",
                    },
                ],
            },
            {
                ...validation,
                blocking_errors: [
                    {
                        field_path: "tracks",
                        code: "value_out_of_range",
                        message_key: "configuration.fix.value_out_of_range",
                    },
                ],
                narrowing_risks: [
                    {
                        code: "all_sources_disabled",
                        condition_key: "configuration.risk.condition",
                        consequence_key: "configuration.risk.consequence",
                    },
                ],
            },
        ]) {
            vi.mocked(invoke).mockResolvedValueOnce(malformed);
            await expect(
                api.validateConfiguration({
                    contract_version: 1,
                    configuration,
                }),
            ).rejects.toMatchObject({
                code: "internal.desktop_contract_mismatch",
            });
        }

        vi.mocked(invoke).mockResolvedValueOnce({
            ...view,
            revision: 2,
            normalized_config_hash: "b".repeat(64),
        });
        await expect(api.saveConfiguration(saveInput)).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });

        vi.mocked(invoke).mockResolvedValueOnce({
            ...view,
            normalized_config_hash: "b".repeat(64),
        });
        await expect(api.configuration()).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
    });

    it("rejects configuration payloads outside every mirrored wire bound", () => {
        const base = {
            contract_version: 1,
            tracks: [{ id: "ai_agents", name: "AI 智能体", enabled: true }],
            include_expression: "AI AND Agent",
            exclude_expression: "deprecated",
            source_preferences: [
                {
                    source_kind: "rss",
                    identifier: "https://example.invalid/feed.xml",
                    enabled: true,
                    trust: 90,
                },
            ],
            refresh_enabled: true,
            refresh_interval_minutes: 60,
            minimum_trust: 40,
            maximum_trust: 100,
            alert_threshold: 80,
            quiet_hours: { enabled: false, start: "22:00", end: "07:00" },
            notification_frequency: { enabled: false, max_per_24h: null },
            active_from: null,
            active_until: null,
        } as const;
        expect(isAttentionConfigurationV1(base)).toBe(true);
        const source = base.source_preferences[0];
        for (const corrupted of [
            { ...base, tracks: [{ ...base.tracks[0], name: "界".repeat(65) }] },
            { ...base, tracks: [base.tracks[0], base.tracks[0]] },
            { ...base, include_expression: "x".repeat(513) },
            { ...base, source_preferences: Array(65).fill(source) },
            {
                ...base,
                source_preferences: [
                    {
                        ...source,
                        identifier: `https://example.invalid/${"x".repeat(2048)}`,
                    },
                ],
            },
            {
                ...base,
                notification_frequency: { enabled: true, max_per_24h: 0 },
            },
            {
                ...base,
                notification_frequency: { enabled: false, max_per_24h: 1 },
            },
            { ...base, minimum_trust: 90, maximum_trust: 80 },
            {
                ...base,
                quiet_hours: { enabled: true, start: "22:00", end: "22:00" },
            },
        ]) {
            expect(isAttentionConfigurationV1(corrupted)).toBe(false);
        }
    });

    it("distinguishes a bounded local timeout from a contract mismatch", async () => {
        vi.mocked(invoke).mockImplementation(
            () => new Promise<never>(() => undefined),
        );
        const api = createTauriDesktopApi((onTimeout) => {
            queueMicrotask(onTimeout);
            return () => undefined;
        });
        await expect(api.setupProgress()).rejects.toMatchObject({
            code: "timeout.desktop_command",
        });
    });

    it("uses exact source commands and rejects unsafe or contradictory source DTOs", async () => {
        const source = {
            contract_version: 1,
            source_id: "source:0123456789abcdef01234567",
            source_kind: "rss_atom",
            display_url: "https://example.com/feed.xml",
            enabled: true,
            revision: 1,
            created_at: "2026-08-18T01:00:00.000Z",
            updated_at: "2026-08-18T01:00:00.000Z",
            last_success_at: null,
            freshness: null,
            status: "ready",
            retryability: "never",
            next_allowed_at: null,
        } as const;
        vi.mocked(invoke)
            .mockResolvedValueOnce(source)
            .mockResolvedValueOnce({
                contract_version: 1,
                items: [source],
                next_cursor: null,
            });
        const api = createTauriDesktopApi();
        const input = {
            contract_version: 1,
            source_kind: "rss_atom",
            url: "https://example.com/feed.xml?private=omitted",
            expected_configuration_revision: 1,
            idempotency_key: "source-save-1",
        } as const;
        await expect(api.saveSource(input)).resolves.toEqual(source);
        expect(invoke).toHaveBeenNthCalledWith(1, "save_source_v1", { input });
        await expect(api.querySources(null, 100)).resolves.toMatchObject({
            items: [source],
        });
        expect(invoke).toHaveBeenNthCalledWith(2, "query_sources_v1", {
            cursor: null,
            limit: 100,
        });

        const automaticRetry = {
            ...source,
            status: "retry_wait",
            retryability: "automatic",
            next_allowed_at: "2026-08-18T01:01:00.000Z",
        } as const;
        vi.mocked(invoke).mockResolvedValueOnce(automaticRetry);
        await expect(api.saveSource(input)).resolves.toEqual(automaticRetry);

        for (const malformed of [
            { ...source, extra: true },
            { ...source, display_url: "https://example.com/feed.xml?secret=1" },
            {
                ...source,
                status: "retry_wait",
                retryability: "after",
                next_allowed_at: null,
            },
            { ...source, revision: Number.MAX_SAFE_INTEGER + 1 },
        ]) {
            vi.mocked(invoke).mockResolvedValueOnce(malformed);
            await expect(api.saveSource(input)).rejects.toMatchObject({
                code: "internal.desktop_contract_mismatch",
            });
        }
    });

    it("uses the three exact RSS sync commands and preserves task identity", async () => {
        const taskId = "task:0123456789abcdef01234567";
        const sourceId = "source:0123456789abcdef01234567";
        const input = {
            contract_version: 1,
            target: { kind: "source_id", source_id: sourceId },
            idempotency_key: "manual-sync-intent-1",
            foreground_budget_ms: 30_000,
        } as const;
        const taskRef = {
            contract_version: 1,
            task_id: taskId,
            state: "queued",
            revision: 1,
        } as const;
        const sourceResult = {
            contract_version: 1,
            source_id: sourceId,
            source_revision: 1,
            state: "running",
            last_success_at: null,
            error_code: null,
            next_allowed_at: null,
            updated_at: "2026-08-18T08:00:01Z",
        } as const;
        const task = {
            contract_version: 1,
            task_id: taskId,
            target: input.target,
            state: "running",
            revision: 2,
            created_at: "2026-08-18T08:00:00Z",
            started_at: "2026-08-18T08:00:01Z",
            finished_at: null,
            updated_at: "2026-08-18T08:00:01Z",
            error_summary: null,
            result_ref: "run:0123456789abcdef01234567",
            sources: [sourceResult],
        } as const;
        const health = {
            contract_version: 1,
            latest_task: task,
            pending_task_count: 1,
            last_success_at: null,
            freshness: null,
            source_results: [sourceResult],
            readiness: {
                contract_version: 1,
                required_source_kinds: ["rss_atom"],
                status: "syncing",
                sources: [
                    {
                        contract_version: 1,
                        source_id: sourceId,
                        source_kind: "rss_atom",
                        status: "syncing",
                        last_success_at: null,
                        next_allowed_at: null,
                    },
                ],
            },
        } as const;
        vi.mocked(invoke)
            .mockResolvedValueOnce(taskRef)
            .mockResolvedValueOnce(task)
            .mockResolvedValueOnce(health);

        const api = createTauriDesktopApi();
        await expect(api.startSync(input)).resolves.toEqual(taskRef);
        await expect(api.task(taskId)).resolves.toEqual(task);
        await expect(api.syncHealth()).resolves.toEqual(health);

        expect(invoke).toHaveBeenNthCalledWith(1, "start_sync_v1", { input });
        expect(invoke).toHaveBeenNthCalledWith(2, "task_v1", { taskId });
        expect(invoke).toHaveBeenNthCalledWith(3, "sync_health_v1");
    });

    it("fails closed for malformed or contradictory sync contracts", async () => {
        const api = createTauriDesktopApi();
        const taskId = "task:0123456789abcdef01234567";
        const sourceId = "source:0123456789abcdef01234567";
        const input = {
            contract_version: 1,
            target: { kind: "source_id", source_id: sourceId },
            idempotency_key: "manual-sync-intent-2",
            foreground_budget_ms: 10_000,
        } as const;

        await expect(
            api.startSync({ ...input, foreground_budget_ms: 30_001 }),
        ).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
        await expect(
            api.startSync({ ...input, foreground_budget_ms: 29_999 }),
        ).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
        expect(invoke).not.toHaveBeenCalled();

        vi.mocked(invoke).mockResolvedValueOnce({
            contract_version: 1,
            task_id: "task:fedcba9876543210fedcba98",
            target: input.target,
            state: "queued",
            revision: 1,
            created_at: "2026-08-18T08:00:00Z",
            started_at: null,
            finished_at: null,
            updated_at: "2026-08-18T08:00:00Z",
            error_summary: null,
            result_ref: "run:0123456789abcdef01234567",
            sources: [],
        });
        await expect(api.task(taskId)).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });

        vi.mocked(invoke).mockResolvedValueOnce({
            contract_version: 1,
            latest_task: null,
            pending_task_count: 0,
            last_success_at: null,
            freshness: null,
            source_results: [],
            readiness: {
                contract_version: 1,
                required_source_kinds: ["rss_atom", "github_release"],
                status: "ready",
                sources: [],
            },
        });
        await expect(api.syncHealth()).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
    });

    it("rejects unknown keys and contradictory task lifecycle fields", () => {
        const task = createTaskSnapshot();
        expect(isTaskSnapshotV1(task)).toBe(true);
        expect(isTaskSnapshotV1({ ...task, result_ref: null })).toBe(true);
        for (const malformed of [
            { ...task, extra: true },
            { ...task, contract_version: 2 },
            { ...task, revision: Number.MAX_SAFE_INTEGER + 1 },
            { ...task, state: "running", finished_at: task.finished_at },
            { ...task, state: "failed", error_summary: null },
            { ...task, result_ref: "run:not-a-run-id" },
            { ...task, updated_at: "2026-08-18T08:00:00+00:00" },
            { ...task, updated_at: "2024-06-30T23:59:60Z" },
            {
                ...task,
                sources: [
                    {
                        ...task.sources[0],
                        state: "retry_wait",
                        error_code: null,
                        next_allowed_at: null,
                    },
                ],
            },
            {
                ...task,
                sources: [
                    {
                        ...task.sources[0],
                        state: "failed",
                        last_success_at: null,
                        error_code: "network.source",
                    },
                ],
            },
        ]) {
            expect(isTaskSnapshotV1(malformed)).toBe(false);
        }
    });

    it("rejects contradictory sync health aggregate fields", () => {
        const task = createTaskSnapshot();
        const health = createSyncHealthSummary({
            latest_task: task,
            source_results: task.sources,
        });
        expect(isSyncHealthSummaryV1(health)).toBe(true);
        expect(
            isSyncHealthSummaryV1({
                ...health,
                readiness: {
                    ...health.readiness,
                    required_source_kinds: ["rss_atom", "github_release"],
                },
            }),
        ).toBe(false);
        expect(
            isSyncHealthSummaryV1({
                ...health,
                source_results: [{ ...task.sources[0], source_revision: 2 }],
            }),
        ).toBe(false);
        expect(
            isSyncHealthSummaryV1({
                ...health,
                pending_task_count: 1,
            }),
        ).toBe(false);
    });

    it("times out start without changing or replaying the idempotency intent", async () => {
        vi.mocked(invoke).mockImplementation(
            () => new Promise<never>(() => undefined),
        );
        const api = createTauriDesktopApi((onTimeout) => {
            queueMicrotask(onTimeout);
            return () => undefined;
        });
        const input = {
            contract_version: 1,
            target: { kind: "all_enabled_rss_atom" },
            idempotency_key: "same-intent-after-timeout",
            foreground_budget_ms: 30_000,
        } as const;

        await expect(api.startSync(input)).rejects.toMatchObject({
            code: "timeout.desktop_command",
        });
        expect(invoke).toHaveBeenCalledOnce();
        expect(invoke).toHaveBeenCalledWith("start_sync_v1", { input });
    });

    it("accepts an exact sync result and rejects contradictory zero-result payloads", async () => {
        const base = createSyncResultPage();
        const source = {
            ...base.summary.sources[0],
            counts: { inserted: 0, updated: 0, skipped: 1, failed: 0 },
        };
        const page = createSyncResultPage({
            summary: {
                ...base.summary,
                outcome: "succeeded_zero_results",
                counts: { inserted: 0, updated: 0, skipped: 1, failed: 0 },
                sources: [source],
            },
            items: [],
        });
        expect(isSyncResultPageV1(page)).toBe(true);
        expect(
            isSyncResultPageV1({
                ...page,
                items: [
                    {
                        ...base.items[0],
                        sync_run_id: page.summary.sync_run_id,
                        source_id: page.summary.sources[0].source_id,
                        original_title: "Contradiction",
                    },
                ],
            }),
        ).toBe(false);

        vi.mocked(invoke).mockResolvedValue(page);
        const input = {
            contract_version: 1,
            sync_run_id: page.summary.sync_run_id,
            cursor: null,
            limit: 25,
        } as const;
        await expect(
            createTauriDesktopApi().getSyncResult(input),
        ).resolves.toEqual(page);
        expect(invoke).toHaveBeenCalledWith("get_sync_result_v1", { input });
    });

    it("accepts zero-item partial results and rejects contradictory result projections", () => {
        const base = createSyncResultPage();
        const succeededSource = {
            ...base.summary.sources[0],
            publisher: "success.example",
            counts: { inserted: 0, updated: 0, skipped: 1, failed: 0 },
        } as const;
        const failedSource = {
            ...base.summary.sources[0],
            source_id: "source:fedcba9876543210fedcba98",
            source_revision: 3,
            publisher: "failed.example",
            status: "failed",
            counts: { inserted: 0, updated: 0, skipped: 0, failed: 1 },
            error_code: "network.source",
        } as const;
        const page = createSyncResultPage({
            summary: {
                ...base.summary,
                outcome: "partially_succeeded",
                counts: { inserted: 0, updated: 0, skipped: 1, failed: 1 },
                sources: [succeededSource, failedSource],
            },
            items: [],
        });
        expect(isSyncResultPageV1(page)).toBe(true);

        for (const malformed of [
            {
                ...page,
                summary: {
                    ...page.summary,
                    sources: [succeededSource, succeededSource],
                },
            },
            {
                ...page,
                summary: {
                    ...page.summary,
                    counts: { ...page.summary.counts, skipped: 2 },
                },
            },
            {
                ...page,
                summary: {
                    ...page.summary,
                    sources: [
                        { ...succeededSource, error_code: "network.source" },
                        failedSource,
                    ],
                },
            },
            {
                ...page,
                summary: {
                    ...page.summary,
                    started_at: "2026-08-18T08:00:02Z",
                },
            },
            {
                ...page,
                summary: {
                    ...page.summary,
                    counts: {
                        ...page.summary.counts,
                        failed: 0x1_0000_0000,
                    },
                },
            },
        ]) {
            expect(isSyncResultPageV1(malformed)).toBe(false);
        }
    });

    it("binds result items and cursors to their authoritative source and run", () => {
        const base = createSyncResultPage();
        const runId = base.summary.sync_run_id;
        const sourceId = base.summary.sources[0].source_id;
        const item = {
            ...base.items[0],
            sync_run_id: runId,
            source_id: sourceId,
            published_at: "2016-12-31T23:59:60Z",
        } as const;
        const cursor = encodeSyncResultCursor(item);
        const page = createSyncResultPage({
            summary: {
                ...base.summary,
                sync_run_id: runId,
                sources: [
                    {
                        ...base.summary.sources[0],
                        source_id: sourceId,
                    },
                ],
            },
            items: [item],
            next_cursor: cursor,
        });
        expect(isSyncResultPageV1(page)).toBe(true);
        expect(
            isSyncResultPageV1({
                ...page,
                items: [{ ...item, publisher: "forged.example" }],
            }),
        ).toBe(false);
        expect(
            isSyncResultPageV1({
                ...page,
                next_cursor: `${cursor.slice(0, -2)}00`,
            }),
        ).toBe(false);
    });

    it("rejects extra get-sync-result input keys before invoking Tauri", async () => {
        const input = {
            contract_version: 1,
            sync_run_id: "run:0123456789abcdef01234567",
            cursor: null,
            limit: 25,
            unexpected: true,
        } as Parameters<
            ReturnType<typeof createTauriDesktopApi>["getSyncResult"]
        >[0];
        await expect(
            createTauriDesktopApi().getSyncResult(input),
        ).rejects.toMatchObject({
            code: "internal.desktop_contract_mismatch",
        });
        expect(invoke).not.toHaveBeenCalled();
    });

    it("normalizes legacy missing intel IDs and rejects malformed additive shapes", async () => {
        const current = createSyncResultPage();
        const legacyItem = Object.fromEntries(
            Object.entries(current.items[0]).filter(
                ([key]) => key !== "intel_item_id",
            ),
        );
        const legacy = { ...current, items: [legacyItem] };
        expect(isSyncResultPageV1(legacy)).toBe(true);
        expect(
            isSyncResultPageV1({
                ...current,
                items: [{ ...current.items[0], intel_item_id: null }],
            }),
        ).toBe(true);
        expect(isSyncResultPageV1(current)).toBe(true);
        expect(
            isSyncResultPageV1({
                ...current,
                items: [{ ...current.items[0], intel_item_id: "intel:BAD" }],
            }),
        ).toBe(false);
        expect(
            isSyncResultPageV1({
                ...current,
                items: [{ ...current.items[0], unexpected: true }],
            }),
        ).toBe(false);

        vi.mocked(invoke).mockResolvedValue(legacy);
        await expect(
            createTauriDesktopApi().getSyncResult({
                contract_version: 1,
                sync_run_id: current.summary.sync_run_id,
                cursor: null,
                limit: 25,
            }),
        ).resolves.toMatchObject({ items: [{ intel_item_id: null }] });
    });
});
