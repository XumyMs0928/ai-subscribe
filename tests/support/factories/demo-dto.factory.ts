import type {
    AttentionConfigurationV1,
    ConfigurationValidationResultV1,
    ConfigurationViewV1,
    DemoCatalogV1,
    DemoEvidenceDetailV1,
    DemoItemV1,
    DemoPageV1,
    DesktopApiError,
    HealthStatusV1,
    IntelFeedItemV1,
    IntelFeedPageV1,
    SetupProgressV1,
    SourceDeliveryReadinessV1,
    SourcePageV1,
    SourceReadinessV1,
    SourceSyncStatusV1,
    SourceViewV1,
    SyncHealthSummaryV1,
    SyncResultPageV1,
    TaskRefV1,
    TaskSnapshotV1,
} from "../../../apps/windows/src/lib/desktop-api/desktop-api";

export function createIntelFeedItem(
    overrides: Partial<IntelFeedItemV1> = {},
): IntelFeedItemV1 {
    return {
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
        ...overrides,
    };
}

export function createIntelFeedPage(
    overrides: Partial<IntelFeedPageV1> = {},
): IntelFeedPageV1 {
    const stream = overrides.stream ?? "high_value";
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
        as_of_ms: 1_787_216_400_000,
        items: [createIntelFeedItem({ stream_disposition: stream })],
        next_cursor: null,
        ...overrides,
    };
}

export function createSourceSyncStatus(
    overrides: Partial<SourceSyncStatusV1> = {},
): SourceSyncStatusV1 {
    return {
        contract_version: 1,
        source_id: "source:0123456789abcdef01234567",
        source_revision: 1,
        state: "succeeded",
        last_success_at: "2026-08-18T02:00:00.000Z",
        error_code: null,
        next_allowed_at: null,
        updated_at: "2026-08-18T02:00:00.000Z",
        ...overrides,
    };
}

export function createTaskSnapshot(
    overrides: Partial<TaskSnapshotV1> = {},
): TaskSnapshotV1 {
    const state = overrides.state ?? "succeeded";
    const lifecycle: Record<
        TaskSnapshotV1["state"],
        Pick<
            TaskSnapshotV1,
            "started_at" | "finished_at" | "error_summary" | "sources"
        >
    > = {
        queued: {
            started_at: null,
            finished_at: null,
            error_summary: null,
            sources: [
                createSourceSyncStatus({
                    state: "queued",
                    last_success_at: null,
                }),
            ],
        },
        running: {
            started_at: "2026-08-18T01:59:59.000Z",
            finished_at: null,
            error_summary: null,
            sources: [
                createSourceSyncStatus({
                    state: "running",
                    last_success_at: null,
                }),
            ],
        },
        retry_wait: {
            started_at: "2026-08-18T01:59:59.000Z",
            finished_at: null,
            error_summary: "rate_limited.source",
            sources: [
                createSourceSyncStatus({
                    state: "retry_wait",
                    last_success_at: null,
                    error_code: "rate_limited.source",
                    next_allowed_at: "2099-08-18T02:00:00.000Z",
                }),
            ],
        },
        succeeded: {
            started_at: "2026-08-18T01:59:59.000Z",
            finished_at: "2026-08-18T02:00:00.000Z",
            error_summary: null,
            sources: [createSourceSyncStatus()],
        },
        partially_succeeded: {
            started_at: "2026-08-18T01:59:59.000Z",
            finished_at: "2026-08-18T02:00:00.000Z",
            error_summary: "source.partial_failure",
            sources: [
                createSourceSyncStatus(),
                createSourceSyncStatus({
                    source_id: "source:fedcba9876543210fedcba98",
                    state: "failed",
                    last_success_at: null,
                    error_code: "network.source",
                }),
            ],
        },
        failed: {
            started_at: "2026-08-18T01:59:59.000Z",
            finished_at: "2026-08-18T02:00:00.000Z",
            error_summary: "source.sync_failed",
            sources: [
                createSourceSyncStatus({
                    state: "failed",
                    last_success_at: null,
                    error_code: "network.source",
                }),
            ],
        },
        cancelled: {
            started_at: "2026-08-18T01:59:59.000Z",
            finished_at: "2026-08-18T02:00:00.000Z",
            error_summary: "internal.unexpected",
            sources: [
                createSourceSyncStatus({
                    state: "cancelled",
                    last_success_at: null,
                    error_code: "internal.unexpected",
                }),
            ],
        },
    };
    return {
        contract_version: 1,
        task_id: "task:0123456789abcdef01234567",
        target: { kind: "all_enabled_rss_atom" },
        state,
        revision: 3,
        created_at: "2026-08-18T01:59:58.000Z",
        started_at: lifecycle[state].started_at,
        finished_at: lifecycle[state].finished_at,
        updated_at: "2026-08-18T02:00:00.000Z",
        error_summary: lifecycle[state].error_summary,
        result_ref: "run:0123456789abcdef01234567",
        sources: lifecycle[state].sources,
        ...overrides,
    };
}

export function createTaskRef(overrides: Partial<TaskRefV1> = {}): TaskRefV1 {
    return {
        contract_version: 1,
        task_id: "task:0123456789abcdef01234567",
        state: "queued",
        revision: 1,
        ...overrides,
    };
}

export function createSyncResultPage(
    overrides: Partial<SyncResultPageV1> = {},
): SyncResultPageV1 {
    const syncRunId =
        overrides.summary?.sync_run_id ?? "run:0123456789abcdef01234567";
    return {
        ...overrides,
        contract_version: 1,
        summary: {
            contract_version: 1,
            sync_run_id: syncRunId,
            task_id: "task:0123456789abcdef01234567",
            outcome: "succeeded_with_results",
            started_at: "2026-08-18T01:59:59.000Z",
            finished_at: "2026-08-18T02:00:00.000Z",
            counts: { inserted: 1, updated: 0, skipped: 0, failed: 0 },
            sources: [
                {
                    contract_version: 1,
                    source_id: "source:0123456789abcdef01234567",
                    source_revision: 2,
                    source_kind: "rss_atom",
                    publisher: "example.com",
                    status: "succeeded",
                    counts: { inserted: 1, updated: 0, skipped: 0, failed: 0 },
                    error_code: null,
                },
            ],
            ...overrides.summary,
        },
        items: overrides.items ?? [
            {
                contract_version: 1,
                result_item_id: "result:0123456789abcdef01234567",
                sync_run_id: syncRunId,
                source_id: "source:0123456789abcdef01234567",
                intel_item_id:
                    "intel:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                source_kind: "rss_atom",
                publisher: "example.com",
                original_title: "Rust release",
                published_at: "2026-08-18T01:00:00.000Z",
                collected_at: "2026-08-18T02:00:00.000Z",
                original_url: "https://example.com/rust-release",
                disposition: "inserted",
            },
        ],
        next_cursor: overrides.next_cursor ?? null,
    };
}

export function encodeSyncResultCursor(
    item: SyncResultPageV1["items"][number],
) {
    const payload = [
        item.sync_run_id,
        item.source_id,
        item.collected_at,
        item.result_item_id,
    ].join("\u001f");
    return `cursor:${Array.from(new TextEncoder().encode(payload), (byte) =>
        byte.toString(16).padStart(2, "0"),
    ).join("")}`;
}

export function createSourceReadiness(
    overrides: Partial<SourceReadinessV1> = {},
): SourceReadinessV1 {
    return {
        contract_version: 1,
        source_id: "source:0123456789abcdef01234567",
        source_kind: "rss_atom",
        status: "available",
        last_success_at: "2026-08-18T02:00:00.000Z",
        next_allowed_at: null,
        ...overrides,
    };
}

export function createSourceDeliveryReadiness(
    overrides: Partial<SourceDeliveryReadinessV1> = {},
): SourceDeliveryReadinessV1 {
    return {
        contract_version: 1,
        required_source_kinds: ["rss_atom"],
        status: "ready",
        sources: [createSourceReadiness()],
        ...overrides,
    };
}

export function createSyncHealthSummary(
    overrides: Partial<SyncHealthSummaryV1> = {},
): SyncHealthSummaryV1 {
    return {
        contract_version: 1,
        latest_task: null,
        pending_task_count: 0,
        last_success_at: "2026-08-18T02:00:00.000Z",
        freshness: "fresh",
        source_results: [],
        readiness: createSourceDeliveryReadiness(),
        ...overrides,
    };
}

export function createSourceView(
    overrides: Partial<SourceViewV1> = {},
): SourceViewV1 {
    return {
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
        ...overrides,
    };
}

export function createSourcePage(
    overrides: Partial<SourcePageV1> = {},
): SourcePageV1 {
    return {
        contract_version: 1,
        items: [createSourceView()],
        next_cursor: null,
        ...overrides,
    };
}

export function createAttentionConfiguration(
    overrides: Partial<AttentionConfigurationV1> = {},
): AttentionConfigurationV1 {
    return {
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
        ...overrides,
    };
}

export function createConfigurationView(
    overrides: Partial<ConfigurationViewV1> = {},
): ConfigurationViewV1 {
    return {
        contract_version: 1,
        revision: 1,
        validator_version: "attention-configuration-v1",
        normalized_config_hash:
            "bcd1f22825c821003d536ebee4afe3312740917f54a463d8e1a4fd8b90e8f7fe",
        configuration: createAttentionConfiguration(),
        updated_at_ms: 1_787_000_000_000,
        ...overrides,
    };
}

export function createConfigurationValidationResult(
    overrides: Partial<ConfigurationValidationResultV1> = {},
): ConfigurationValidationResultV1 {
    return {
        contract_version: 1,
        blocking_errors: [],
        narrowing_risks: [],
        validator_version: "attention-configuration-v1",
        normalized_config_hash:
            "bcd1f22825c821003d536ebee4afe3312740917f54a463d8e1a4fd8b90e8f7fe",
        validation_receipt: null,
        ...overrides,
    };
}

const DEFAULT_ITEMS: readonly DemoItemV1[] = [
    {
        id: "demo:openai-agents-sdk-001",
        data_origin: "demo",
        publisher: "OpenAI",
        title: "Agents SDK 发布新的会话追踪能力",
        track: "AI Agent",
        summary: "固定演示样本：展示如何从来源事实形成可核验的情报摘要。",
        original_url: "https://openai.com/agents-sdk/",
        importance: "high",
        ai_status: "generated",
        published_at: "2026-07-01T08:00:00Z",
        collected_at: "2026-07-01T09:00:00Z",
    },
    {
        id: "demo:rust-197-001",
        data_origin: "demo",
        publisher: "Rust Project",
        title: "Rust 1.97 提升工具链诊断体验",
        track: "开发工具",
        summary: "固定演示样本：说明版本变化、影响范围与原始来源。",
        original_url: "https://www.rust-lang.org/tools/install",
        importance: "medium",
        ai_status: "generated",
        published_at: "2026-06-20T10:00:00Z",
        collected_at: "2026-06-20T10:30:00Z",
    },
    {
        id: "demo:local-model-001",
        data_origin: "demo",
        publisher: "AI Subscribe Demo",
        title: "本地模型推理的内存占用进一步下降",
        track: "本地模型",
        summary: "固定演示样本：帮助用户理解离线情报浏览和基础筛选。",
        original_url: "https://example.com/local-model-demo",
        importance: "medium",
        ai_status: "generated",
        published_at: "2026-06-10T02:00:00Z",
        collected_at: "2026-06-10T03:00:00Z",
    },
];

export function createDemoItem(
    overrides: Partial<DemoEvidenceDetailV1> = {},
): DemoEvidenceDetailV1 {
    return {
        ...DEFAULT_ITEMS[0],
        contract_version: 1,
        dataset_id: "demo-v1",
        what_happened: "OpenAI Agents SDK 的演示版本增加了会话追踪能力。",
        why_it_matters: "更清晰的会话轨迹有助于定位多步骤 Agent 的失败位置。",
        possible_impact: "团队可用更少的手工日志还原复杂 Agent 的执行过程。",
        facts: ["演示来源声明了新的会话追踪能力。"],
        rule_reasons: ["命中 AI Agent 赛道"],
        ai_content: "演示 AI 生成：该能力可能降低复杂 Agent 的排障成本。",
        ai_confidence_percent: 88,
        provenance: {
            source_kind: "official_release",
            publisher: "OpenAI",
            author: null,
            original_title: "Agents SDK session tracing demo",
            original_url: "https://openai.com/",
            published_at: "2026-07-01T08:00:00Z",
            collected_at: "2026-07-01T09:00:00Z",
            first_discovered_at: "2026-07-01T09:00:00Z",
            last_updated_at: "2026-07-01T09:00:00Z",
            availability_status: "available",
            deterministic_association_basis: "demo_fixture_id",
        },
        ...overrides,
    };
}

export function createSetupProgress(
    overrides: Partial<SetupProgressV1> = {},
): SetupProgressV1 {
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
            default_track_ids: [
                "ai_agents",
                "foundation_models",
                "local_models",
            ],
            default_source_example_ids: [
                "github_releases",
                "arxiv_topics",
                "rss_feeds",
            ],
            default_refresh_cadence: "manual",
            tracks: [
                { id: "ai_agents", label: "AI 智能体", is_demo: true },
                { id: "foundation_models", label: "基础模型", is_demo: true },
                { id: "local_models", label: "本地模型", is_demo: true },
            ],
            source_examples: [
                {
                    id: "github_releases",
                    label: "GitHub Release 示例",
                    is_demo: true,
                },
                { id: "arxiv_topics", label: "arXiv 主题示例", is_demo: true },
                { id: "rss_feeds", label: "RSS/Atom 来源示例", is_demo: true },
            ],
            refresh_cadences: [
                { id: "manual", label: "仅手动刷新", is_demo: false },
                { id: "daily", label: "每日一次", is_demo: false },
            ],
        },
        saved_config: {
            track_ids: ["ai_agents", "foundation_models", "local_models"],
            source_example_ids: [
                "github_releases",
                "arxiv_topics",
                "rss_feeds",
            ],
            refresh_cadence: "manual",
            ai_data_disclosure_acknowledged: false,
        },
        ...overrides,
    };
}

export function createDemoCatalog(
    overrides: Partial<DemoCatalogV1> = {},
): DemoCatalogV1 {
    return {
        contract_version: 1,
        dataset_id: "demo-v1",
        items: DEFAULT_ITEMS.map((item) => ({ ...item })),
        ...overrides,
    };
}

export function createDemoPage(
    overrides: Partial<DemoPageV1> = {},
): DemoPageV1 {
    return {
        ...createDemoCatalog(),
        next_cursor: null,
        ...overrides,
    };
}

export function createHealthStatus(
    overrides: Partial<HealthStatusV1> = {},
): HealthStatusV1 {
    return {
        contract_version: 1,
        status: "ok",
        checked_at: "2026-06-22T09:30:00Z",
        ...overrides,
    };
}

export function createDesktopApiError(
    overrides: Partial<DesktopApiError> = {},
): DesktopApiError {
    return {
        contract_version: 1,
        code: "internal.demo_fixture_error",
        category: "internal",
        message_key: "error.demo_fixture",
        retryability: "retryable",
        source_id: null,
        task_id: null,
        details_allowlisted: "deterministic Playwright fixture error",
        correlation_id: "pw-demo-error-001",
        ...overrides,
    };
}
