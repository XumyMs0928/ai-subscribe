export interface HealthStatusV1 {
    readonly contract_version: 1;
    readonly status: "ok";
    readonly checked_at: string | null;
}

export interface DesktopApiError {
    readonly contract_version: 1;
    readonly code: string;
    readonly category: string;
    readonly message_key: string;
    readonly retryability: string;
    readonly source_id: string | null;
    readonly task_id: string | null;
    readonly details_allowlisted: string;
    readonly correlation_id: string;
}

export interface DesktopApi {
    health(): Promise<HealthStatusV1>;
    demoBootstrap(): Promise<DemoCatalogV1>;
    demoSearch(query: string, track: string | null): Promise<DemoCatalogV1>;
    demoList(cursor: string | null, limit: number): Promise<DemoPageV1>;
    demoFilter(
        track: string,
        cursor: string | null,
        limit: number,
    ): Promise<DemoPageV1>;
    demoDetail(id: string): Promise<DemoEvidenceDetailV1>;
    setupProgress(): Promise<SetupProgressV1>;
    saveSetupStep(input: SaveSetupStepInputV1): Promise<SetupProgressV1>;
    configuration(): Promise<ConfigurationViewV1>;
    validateConfiguration(
        input: ValidateConfigurationInputV1,
    ): Promise<ConfigurationValidationResultV1>;
    saveConfiguration(
        input: SaveConfigurationInputV1,
    ): Promise<ConfigurationViewV1>;
    saveSource(input: SaveSourceInputV1): Promise<SourceViewV1>;
    querySources(cursor: string | null, limit: number): Promise<SourcePageV1>;
    startSync(input: StartSyncInputV1): Promise<TaskRefV1>;
    task(taskId: string): Promise<TaskSnapshotV1>;
    syncHealth(): Promise<SyncHealthSummaryV1>;
    getSyncResult(input: GetSyncResultInputV1): Promise<SyncResultPageV1>;
    queryIntelFeed(input: QueryIntelFeedInputV1): Promise<IntelFeedPageV1>;
    queryIntelEvidenceDetail(
        intelItemId: string,
    ): Promise<IntelEvidenceDetailV1>;
    openIntelOriginal(
        intelItemId: string,
        provenanceId: string,
    ): Promise<OpenOriginalReceiptV1>;
}

export type IntelFeedStreamV1 = "high_value" | "ordinary_candidate";
export type IntelFeedTimeWindowV1 =
    "all_time" | "last_24h" | "last_7d" | "last_30d";
export type IntelFeedSortV1 = "score_desc";

export interface IntelFeedFiltersV1 {
    readonly track_ids: readonly string[];
    readonly source_ids: readonly string[];
    readonly time_window: IntelFeedTimeWindowV1;
    readonly importance: readonly ("low" | "medium" | "high")[];
}

export interface QueryIntelFeedInputV1 {
    readonly contract_version: 1;
    readonly stream: IntelFeedStreamV1;
    readonly filters: IntelFeedFiltersV1;
    readonly sort: IntelFeedSortV1;
    readonly cursor: string | null;
    readonly limit: number;
}

export interface IntelFeedItemV1 {
    readonly contract_version: 1;
    readonly intel_item_id: string;
    readonly source_id: string;
    readonly source_kind: "rss_atom";
    readonly publisher: string;
    readonly title: string;
    readonly source_excerpt: string | null;
    readonly excerpt_truncated: boolean;
    readonly published_at: string | null;
    readonly collected_at: string;
    readonly importance: "low" | "medium" | "high";
    readonly score: number;
    readonly matched_track_ids: readonly string[];
    readonly stream_disposition: IntelFeedStreamV1;
    readonly ai_status: "unavailable";
}

export interface IntelFeedPageV1 {
    readonly contract_version: 1;
    readonly stream: IntelFeedStreamV1;
    readonly filters: IntelFeedFiltersV1;
    readonly sort: IntelFeedSortV1;
    readonly rule_version: "rss-intelligence-value-v1";
    readonly configuration_revision: number;
    readonly configuration_hash: string;
    readonly as_of_ms: number;
    readonly items: readonly IntelFeedItemV1[];
    readonly next_cursor: string | null;
}

export type RuleEvidenceStatusV1 = "current" | "unavailable" | "stale";
export type AiEvidenceStatusV1 = "unavailable";
export type ProvenanceRoleV1 = "primary" | "associated";
export type AssociationEvidenceStatusV1 = "complete" | "incomplete";

export interface QueryIntelEvidenceDetailInputV1 {
    readonly contract_version: 1;
    readonly intel_item_id: string;
}

export interface OpenIntelOriginalInputV1 extends QueryIntelEvidenceDetailInputV1 {
    readonly provenance_id: string;
}

export interface SourceFactsV1 {
    readonly intel_item_id: string;
    readonly fact_revision: number;
    readonly content_hash: string;
    readonly content_state: "metadata_only";
    readonly publisher: string;
    readonly title: string;
    readonly source_summary: string | null;
    readonly published_at: string | null;
    readonly collected_at: string;
}

export interface RuleFactorV1 {
    readonly factor:
        | "track"
        | "source_trust"
        | "freshness"
        | "technical_impact"
        | "user_rule";
    readonly points: number;
    readonly reason_codes: readonly string[];
}

export interface FilterReasonV1 {
    readonly code: string;
    readonly actual: number | null;
    readonly threshold: number | null;
}

export interface RuleExplanationV1 {
    readonly rule_version: "rss-intelligence-value-v1";
    readonly configuration_revision: number;
    readonly configuration_hash: string;
    readonly evaluated_at_ms: number;
    readonly score: number;
    readonly importance: "low" | "medium" | "high";
    readonly disposition: IntelFeedStreamV1;
    readonly matched_track_ids: readonly string[];
    readonly factors: readonly RuleFactorV1[];
    readonly filter_reasons: readonly FilterReasonV1[];
}

export interface IntelProvenanceV1 {
    readonly provenance_id: string;
    readonly intel_item_id: string;
    readonly role: ProvenanceRoleV1;
    readonly source_id: string;
    readonly source_kind: "rss_atom";
    readonly publisher: string;
    readonly author: string | null;
    readonly author_availability:
        "available" | "unavailable" | "unknown_legacy";
    readonly original_title: string;
    readonly display_url: string;
    readonly published_at: string | null;
    readonly collected_at: string;
    readonly first_discovered_at: string;
    readonly last_updated_at: string;
    readonly availability_status:
        "available" | "unavailable" | "unknown_legacy";
    readonly can_open_original: boolean;
}

export interface AssociationEvidenceV1 {
    readonly status: AssociationEvidenceStatusV1;
    readonly issue_code: string | null;
    readonly relation_type: "same_event" | null;
    readonly evidence_basis: "normalized_original_url" | null;
    readonly basis_version: 1 | null;
}

export interface IntelEvidenceDetailV1 {
    readonly contract_version: 1;
    readonly facts: SourceFactsV1;
    readonly rule_status: RuleEvidenceStatusV1;
    readonly rule_issue_code: string | null;
    readonly rule: RuleExplanationV1 | null;
    readonly ai_status: AiEvidenceStatusV1;
    readonly provenance: readonly IntelProvenanceV1[];
    readonly association: AssociationEvidenceV1;
}

export interface OpenOriginalReceiptV1 {
    readonly contract_version: 1;
    readonly intel_item_id: string;
    readonly provenance_id: string;
    readonly status: "requested";
}

export interface SaveSourceInputV1 {
    readonly contract_version: 1;
    readonly source_kind: "rss_atom";
    readonly url: string;
    readonly expected_configuration_revision: number;
    readonly idempotency_key: string;
}

export type SourceStatusV1 = "ready" | "error" | "retry_wait";
export type SourceRetryabilityV1 = "never" | "manual" | "automatic" | "after";

export interface SourceViewV1 {
    readonly contract_version: 1;
    readonly source_id: string;
    readonly source_kind: "rss_atom";
    readonly display_url: string;
    readonly enabled: boolean;
    readonly revision: number;
    readonly created_at: string;
    readonly updated_at: string;
    readonly last_success_at: string | null;
    readonly freshness: string | null;
    readonly status: SourceStatusV1;
    readonly retryability: SourceRetryabilityV1;
    readonly next_allowed_at: string | null;
}

export interface SourcePageV1 {
    readonly contract_version: 1;
    readonly items: readonly SourceViewV1[];
    readonly next_cursor: string | null;
}

export type SyncTargetV1 =
    | { readonly kind: "all_enabled_rss_atom" }
    | { readonly kind: "source_id"; readonly source_id: string };

export interface StartSyncInputV1 {
    readonly contract_version: 1;
    readonly target: SyncTargetV1;
    readonly idempotency_key: string;
    readonly foreground_budget_ms: number;
}

export type TaskStateV1 =
    | "queued"
    | "running"
    | "retry_wait"
    | "succeeded"
    | "partially_succeeded"
    | "failed"
    | "cancelled";

export interface TaskRefV1 {
    readonly contract_version: 1;
    readonly task_id: string;
    readonly state: TaskStateV1;
    readonly revision: number;
}

export interface SourceSyncStatusV1 {
    readonly contract_version: 1;
    readonly source_id: string;
    readonly source_revision: number;
    readonly state: TaskStateV1;
    readonly last_success_at: string | null;
    readonly error_code: string | null;
    readonly next_allowed_at: string | null;
    readonly updated_at: string;
}

export interface TaskSnapshotV1 {
    readonly contract_version: 1;
    readonly task_id: string;
    readonly target: SyncTargetV1;
    readonly state: TaskStateV1;
    readonly revision: number;
    readonly created_at: string;
    readonly started_at: string | null;
    readonly finished_at: string | null;
    readonly updated_at: string;
    readonly error_summary: string | null;
    readonly result_ref: string | null;
    readonly sources: readonly SourceSyncStatusV1[];
}

export type SyncRunOutcomeV1 =
    | "succeeded_with_results"
    | "succeeded_zero_results"
    | "partially_succeeded"
    | "failed";

export type SyncResultDispositionV1 = "inserted" | "updated";

export interface GetSyncResultInputV1 {
    readonly contract_version: 1;
    readonly sync_run_id: string;
    readonly cursor: string | null;
    readonly limit: number;
}

export interface SyncResultCountsV1 {
    readonly inserted: number;
    readonly updated: number;
    readonly skipped: number;
    readonly failed: number;
}

export interface SyncSourceResultV1 {
    readonly contract_version: 1;
    readonly source_id: string;
    readonly source_revision: number;
    readonly source_kind: "rss_atom";
    readonly publisher: string;
    readonly status:
        | "queued"
        | "running"
        | "retry_wait"
        | "succeeded"
        | "failed"
        | "cancelled";
    readonly counts: SyncResultCountsV1;
    readonly error_code: string | null;
}

export interface SyncResultItemV1 {
    readonly contract_version: 1;
    readonly result_item_id: string;
    readonly sync_run_id: string;
    readonly source_id: string;
    readonly intel_item_id: string | null;
    readonly source_kind: "rss_atom";
    readonly publisher: string;
    readonly original_title: string;
    readonly published_at: string | null;
    readonly collected_at: string;
    readonly original_url: string;
    readonly disposition: SyncResultDispositionV1;
}

export interface SyncResultSummaryV1 {
    readonly contract_version: 1;
    readonly sync_run_id: string;
    readonly task_id: string;
    readonly outcome: SyncRunOutcomeV1;
    readonly started_at: string;
    readonly finished_at: string;
    readonly counts: SyncResultCountsV1;
    readonly sources: readonly SyncSourceResultV1[];
}

export interface SyncResultPageV1 {
    readonly contract_version: 1;
    readonly summary: SyncResultSummaryV1;
    readonly items: readonly SyncResultItemV1[];
    readonly next_cursor: string | null;
}

export type SourceReadinessStatusV1 =
    | "not_configured"
    | "available"
    | "syncing"
    | "rate_limited"
    | "failed"
    | "disabled"
    | "retry_wait";

export type DeliveryReadinessStatusV1 =
    "not_configured" | "ready" | "syncing" | "blocked";

export interface SourceReadinessV1 {
    readonly contract_version: 1;
    readonly source_id: string | null;
    readonly source_kind: "rss_atom";
    readonly status: SourceReadinessStatusV1;
    readonly last_success_at: string | null;
    readonly next_allowed_at: string | null;
}

export interface SourceDeliveryReadinessV1 {
    readonly contract_version: 1;
    readonly required_source_kinds: readonly ["rss_atom"];
    readonly status: DeliveryReadinessStatusV1;
    readonly sources: readonly SourceReadinessV1[];
}

export interface SyncHealthSummaryV1 {
    readonly contract_version: 1;
    readonly latest_task: TaskSnapshotV1 | null;
    readonly pending_task_count: number;
    readonly last_success_at: string | null;
    readonly freshness: string | null;
    readonly source_results: readonly SourceSyncStatusV1[];
    readonly readiness: SourceDeliveryReadinessV1;
}

export interface AttentionTrackV1 {
    readonly id: string;
    readonly name: string;
    readonly enabled: boolean;
}

export interface SourcePreferenceV1 {
    readonly source_kind: "rss" | "github" | "arxiv";
    readonly identifier: string;
    readonly enabled: boolean;
    readonly trust: number;
}

export interface AttentionConfigurationV1 {
    readonly contract_version: 1;
    readonly tracks: readonly AttentionTrackV1[];
    readonly include_expression: string;
    readonly exclude_expression: string;
    readonly source_preferences: readonly SourcePreferenceV1[];
    readonly refresh_enabled: boolean;
    readonly refresh_interval_minutes: number;
    readonly minimum_trust: number;
    readonly maximum_trust: number;
    readonly alert_threshold: number;
    readonly quiet_hours: {
        readonly enabled: boolean;
        readonly start: string;
        readonly end: string;
    };
    readonly notification_frequency: {
        readonly enabled: boolean;
        readonly max_per_24h: number | null;
    };
    readonly active_from: string | null;
    readonly active_until: string | null;
}

export type BlockingCodeV1 =
    | "expression_unparseable"
    | "value_out_of_range"
    | "lower_bound_above_upper_bound"
    | "invalid_source_or_unsupported_protocol";
export type NarrowingRiskCodeV1 =
    "all_sources_disabled" | "all_high_trust_candidates_filtered";

export interface ConfigurationValidationReceiptV1 {
    readonly token: string;
    readonly normalized_config_hash: string;
    readonly validator_version: "attention-configuration-v1";
}

export interface ConfigurationValidationResultV1 {
    readonly contract_version: 1;
    readonly blocking_errors: readonly {
        readonly field_path: string;
        readonly code: BlockingCodeV1;
        readonly message_key: string;
    }[];
    readonly narrowing_risks: readonly {
        readonly code: NarrowingRiskCodeV1;
        readonly condition_key: string;
        readonly consequence_key: string;
    }[];
    readonly validator_version: "attention-configuration-v1";
    readonly normalized_config_hash: string;
    readonly validation_receipt: ConfigurationValidationReceiptV1 | null;
}

export interface ValidateConfigurationInputV1 {
    readonly contract_version: 1;
    readonly configuration: AttentionConfigurationV1;
}

export interface SaveConfigurationInputV1 {
    readonly contract_version: 1;
    readonly configuration: AttentionConfigurationV1;
    readonly expected_revision: number;
    readonly expected_normalized_config_hash: string;
    readonly idempotency_key: string;
    readonly validation_receipt: ConfigurationValidationReceiptV1 | null;
}

export interface ConfigurationViewV1 {
    readonly contract_version: 1;
    readonly revision: number;
    readonly validator_version: "attention-configuration-v1";
    readonly normalized_config_hash: string;
    readonly configuration: AttentionConfigurationV1;
    readonly updated_at_ms: number;
}

export const SETUP_STEP_IDS = [
    "tracks",
    "source_examples",
    "refresh_cadence",
    "ai_data_disclosure",
] as const;
export type SetupStepIdV1 = (typeof SETUP_STEP_IDS)[number];
export type SetupStepStatusV1 =
    | "not_started"
    | "in_progress"
    | "skipped"
    | "partially_completed"
    | "completed";
export type SetupActionV1 = "save" | "skip" | "later";

export interface SetupOptionV1 {
    readonly id: string;
    readonly label: string;
    readonly is_demo: boolean;
}

export interface SetupDefaultsV1 {
    readonly contract_version: 1;
    readonly fixture_id: "setup-defaults-v1";
    readonly default_track_ids: readonly string[];
    readonly default_source_example_ids: readonly string[];
    readonly default_refresh_cadence: string;
    readonly tracks: readonly SetupOptionV1[];
    readonly source_examples: readonly SetupOptionV1[];
    readonly refresh_cadences: readonly SetupOptionV1[];
}

export interface SetupStepProgressV1 {
    readonly contract_version: 1;
    readonly step_id: SetupStepIdV1;
    readonly status: SetupStepStatusV1;
    readonly saved_fields_version: number | null;
}

export interface SetupSavedConfigV1 {
    readonly track_ids: readonly string[];
    readonly source_example_ids: readonly string[];
    readonly refresh_cadence: string | null;
    readonly ai_data_disclosure_acknowledged: boolean;
}

export interface SetupProgressV1 {
    readonly contract_version: 1;
    readonly revision: number;
    readonly configuration_revision: number;
    readonly overall_status: SetupStepStatusV1;
    readonly steps: readonly SetupStepProgressV1[];
    readonly next_step_id: SetupStepIdV1 | null;
    readonly defaults: SetupDefaultsV1;
    readonly saved_config: SetupSavedConfigV1;
}

export interface SaveSetupStepInputV1 {
    readonly contract_version: 1;
    readonly step_id: SetupStepIdV1;
    readonly action: SetupActionV1;
    readonly selected_values: readonly string[];
    readonly expected_revision: number;
    readonly expected_configuration_revision: number;
    readonly idempotency_key: string;
}

export interface DemoItemV1 {
    readonly id: string;
    readonly data_origin: "demo";
    readonly publisher: string;
    readonly title: string;
    readonly track: string;
    readonly summary: string;
    readonly original_url: string;
    readonly importance?: ImportanceV1;
    readonly ai_status?: AiStatusV1;
    readonly published_at: string;
    readonly collected_at: string;
}

export type ImportanceV1 = "low" | "medium" | "high";
export type AiStatusV1 = "generated" | "waiting" | "failed" | "unavailable";
export type AvailabilityStatusV1 = "available" | "unavailable";

export interface DemoProvenanceV1 {
    readonly source_kind: string;
    readonly publisher: string;
    readonly author: string | null;
    readonly original_title: string;
    readonly original_url: string;
    readonly published_at: string | null;
    readonly collected_at: string;
    readonly first_discovered_at: string;
    readonly last_updated_at: string;
    readonly availability_status: AvailabilityStatusV1;
    readonly deterministic_association_basis: string | null;
}

export interface DemoEvidenceDetailV1 extends DemoItemV1 {
    readonly contract_version: 1;
    readonly dataset_id: "demo-v1";
    readonly what_happened: string;
    readonly why_it_matters: string;
    readonly possible_impact: string;
    readonly facts: readonly string[];
    readonly rule_reasons: readonly string[];
    readonly ai_content: string;
    readonly ai_confidence_percent: number;
    readonly provenance: DemoProvenanceV1;
}

export interface DemoCatalogV1 {
    readonly contract_version: 1;
    readonly dataset_id: "demo-v1";
    readonly items: readonly DemoItemV1[];
}

export class DesktopContractError extends Error {
    readonly code = "internal.desktop_contract_mismatch";

    constructor() {
        super("The desktop bridge returned an invalid v1 health contract.");
        this.name = "DesktopContractError";
    }
}

export class DesktopTimeoutError extends Error {
    readonly code = "timeout.desktop_command";

    constructor() {
        super("The desktop command did not respond before the local timeout.");
        this.name = "DesktopTimeoutError";
    }
}

export class DesktopCommandError extends Error implements DesktopApiError {
    readonly contract_version: 1;
    readonly code: string;
    readonly category: string;
    readonly message_key: string;
    readonly retryability: string;
    readonly source_id: string | null;
    readonly task_id: string | null;
    readonly details_allowlisted: string;
    readonly correlation_id: string;

    constructor(error: DesktopApiError) {
        super(error.message_key);
        this.name = "DesktopCommandError";
        this.contract_version = error.contract_version;
        this.code = error.code;
        this.category = error.category;
        this.message_key = error.message_key;
        this.retryability = error.retryability;
        this.source_id = error.source_id;
        this.task_id = error.task_id;
        this.details_allowlisted = error.details_allowlisted;
        this.correlation_id = error.correlation_id;
    }
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return !!value && typeof value === "object";
}

function isNullableNonEmptyTrimmed(value: unknown): value is string | null {
    return value === null || isNonEmptyTrimmed(value);
}

function isRfc3339Utc(value: string): boolean {
    if (value.length < 20 || value.length > 64) return false;
    const match =
        /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?Z$/.exec(
            value,
        );
    if (!match) return false;
    const [, yearText, monthText, dayText, hourText, minuteText, secondText] =
        match;
    const year = Number(yearText);
    const month = Number(monthText);
    const day = Number(dayText);
    const hour = Number(hourText);
    const minute = Number(minuteText);
    const second = Number(secondText);
    const leapYear = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
    const days = [
        31,
        leapYear ? 29 : 28,
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    const announcedLeapSecond =
        second === 60 &&
        hour === 23 &&
        minute === 59 &&
        ((month === 6 &&
            day === 30 &&
            [
                1972, 1981, 1982, 1983, 1985, 1992, 1993, 1994, 1997, 2012,
                2015,
            ].includes(year)) ||
            (month === 12 &&
                day === 31 &&
                [
                    1972, 1973, 1974, 1975, 1976, 1977, 1978, 1979, 1987, 1989,
                    1990, 1995, 1998, 2005, 2008, 2016,
                ].includes(year)));
    return (
        month >= 1 &&
        month <= 12 &&
        day >= 1 &&
        day <= days[month - 1] &&
        hour <= 23 &&
        minute <= 59 &&
        (second <= 59 || announcedLeapSecond)
    );
}

function compareRfc3339Utc(left: string, right: string): number {
    const split = (value: string) => {
        const withoutZulu = value.slice(0, -1);
        const [whole, fraction = ""] = withoutZulu.split(".");
        return { whole, fraction };
    };
    const leftParts = split(left);
    const rightParts = split(right);
    if (leftParts.whole !== rightParts.whole) {
        return leftParts.whole < rightParts.whole ? -1 : 1;
    }
    const width = Math.max(
        leftParts.fraction.length,
        rightParts.fraction.length,
    );
    const leftFraction = leftParts.fraction.padEnd(width, "0");
    const rightFraction = rightParts.fraction.padEnd(width, "0");
    if (leftFraction === rightFraction) return 0;
    return leftFraction < rightFraction ? -1 : 1;
}

export interface DemoPageV1 extends DemoCatalogV1 {
    readonly next_cursor: string | null;
}

function isNonEmptyTrimmed(value: unknown): value is string {
    return (
        typeof value === "string" && value.length > 0 && value.trim() === value
    );
}

function isHttpsUrl(value: unknown): value is string {
    if (typeof value !== "string") return false;
    try {
        const parsed = new URL(value);
        return parsed.protocol === "https:" && parsed.hostname.length > 0;
    } catch {
        return false;
    }
}

export function isHealthStatusV1(value: unknown): value is HealthStatusV1 {
    if (!isRecord(value)) return false;
    const record = value;
    return (
        record.contract_version === 1 &&
        record.status === "ok" &&
        Object.hasOwn(record, "checked_at") &&
        (record.checked_at === null ||
            (typeof record.checked_at === "string" &&
                isRfc3339Utc(record.checked_at)))
    );
}

export function isDesktopApiError(value: unknown): value is DesktopApiError {
    if (!isRecord(value)) return false;
    const mappings: Readonly<
        Record<
            string,
            readonly [
                category: string,
                messageKey: string,
                retryability: string,
            ]
        >
    > = {
        "validation.effect_id": ["validation", "error.validation", "never"],
        "validation.idempotency_key": [
            "validation",
            "error.validation",
            "never",
        ],
        "validation.rfc3339_utc": ["validation", "error.validation", "never"],
        "validation.effect_status": ["validation", "error.validation", "never"],
        "validation.secret_lease": ["validation", "error.validation", "never"],
        "validation.setup_input": ["validation", "error.validation", "never"],
        "validation.configuration": ["validation", "error.validation", "never"],
        "validation.stale_validation_receipt": [
            "validation",
            "error.validation",
            "never",
        ],
        "validation.source": ["validation", "error.validation", "never"],
        "not_found.intel_detail": ["not_found", "error.not_found", "never"],
        "conflict.effect_already_reported": [
            "conflict",
            "error.conflict",
            "manual",
        ],
        "conflict.secret_lease_consumed": [
            "conflict",
            "error.conflict",
            "manual",
        ],
        "conflict.setup_revision": ["conflict", "error.conflict", "manual"],
        "conflict.configuration_revision": [
            "conflict",
            "error.conflict",
            "manual",
        ],
        "conflict.source_revision": ["conflict", "error.conflict", "manual"],
        "network.source": ["network", "error.network.source", "automatic"],
        "rate_limited.source": [
            "rate_limited",
            "error.rate_limited.source",
            "after",
        ],
        "source_format.rss_atom": [
            "source_format",
            "error.source_format.rss_atom",
            "never",
        ],
        "storage.setup": ["storage", "error.internal", "never"],
        "storage.configuration": ["storage", "error.internal", "never"],
        "storage.source": ["storage", "error.internal", "never"],
        "migration.setup": ["migration", "error.internal", "never"],
        "migration.source": ["migration", "error.internal", "never"],
        "internal.unexpected": ["internal", "error.internal", "manual"],
    };
    const mapping =
        typeof value.code === "string" ? mappings[value.code] : undefined;
    return (
        hasExactKeys(value, [
            "contract_version",
            "code",
            "category",
            "message_key",
            "retryability",
            "source_id",
            "task_id",
            "details_allowlisted",
            "correlation_id",
        ]) &&
        value.contract_version === 1 &&
        mapping !== undefined &&
        value.category === mapping[0] &&
        value.message_key === mapping[1] &&
        value.retryability === mapping[2] &&
        (value.source_id === null ||
            (typeof value.source_id === "string" &&
                /^source:[0-9a-f]{24}$/.test(value.source_id))) &&
        (value.task_id === null || isTaskId(value.task_id)) &&
        typeof value.details_allowlisted === "string" &&
        value.details_allowlisted.length <= 512 &&
        isNonEmptyTrimmed(value.correlation_id) &&
        value.correlation_id.length <= 128
    );
}

const DETAIL_ONLY_FIELDS = [
    "what_happened",
    "why_it_matters",
    "possible_impact",
    "facts",
    "rule_reasons",
    "ai_content",
    "ai_confidence_percent",
    "provenance",
] as const;

function isDemoItemShape(
    value: unknown,
    allowDetailFields: boolean,
): value is DemoItemV1 {
    if (!isRecord(value)) return false;
    return (
        (allowDetailFields ||
            DETAIL_ONLY_FIELDS.every(
                (field) => !Object.hasOwn(value, field),
            )) &&
        typeof value.id === "string" &&
        /^demo:[A-Za-z0-9._:-]+$/.test(value.id) &&
        value.id.length > 5 &&
        value.id.length <= 128 &&
        value.data_origin === "demo" &&
        isNonEmptyTrimmed(value.publisher) &&
        isNonEmptyTrimmed(value.title) &&
        isNonEmptyTrimmed(value.track) &&
        isNonEmptyTrimmed(value.summary) &&
        isHttpsUrl(value.original_url) &&
        (value.importance === undefined ||
            (typeof value.importance === "string" &&
                ["low", "medium", "high"].includes(value.importance))) &&
        (value.ai_status === undefined ||
            (typeof value.ai_status === "string" &&
                ["generated", "waiting", "failed", "unavailable"].includes(
                    value.ai_status,
                ))) &&
        typeof value.published_at === "string" &&
        isRfc3339Utc(value.published_at) &&
        typeof value.collected_at === "string" &&
        isRfc3339Utc(value.collected_at)
    );
}

export function isDemoItemV1(value: unknown): value is DemoItemV1 {
    return isDemoItemShape(value, false);
}

function isStringList(value: unknown): value is readonly string[] {
    return (
        Array.isArray(value) &&
        value.length > 0 &&
        value.every(isNonEmptyTrimmed)
    );
}

function isDemoProvenanceV1(value: unknown): value is DemoProvenanceV1 {
    return (
        isRecord(value) &&
        isNonEmptyTrimmed(value.source_kind) &&
        isNonEmptyTrimmed(value.publisher) &&
        isNullableNonEmptyTrimmed(value.author) &&
        isNonEmptyTrimmed(value.original_title) &&
        isHttpsUrl(value.original_url) &&
        (value.published_at === null ||
            (typeof value.published_at === "string" &&
                isRfc3339Utc(value.published_at))) &&
        typeof value.collected_at === "string" &&
        isRfc3339Utc(value.collected_at) &&
        typeof value.first_discovered_at === "string" &&
        isRfc3339Utc(value.first_discovered_at) &&
        typeof value.last_updated_at === "string" &&
        isRfc3339Utc(value.last_updated_at) &&
        ["available", "unavailable"].includes(
            String(value.availability_status),
        ) &&
        isNullableNonEmptyTrimmed(value.deterministic_association_basis)
    );
}

export function isDemoEvidenceDetailV1(
    value: unknown,
): value is DemoEvidenceDetailV1 {
    return (
        isRecord(value) &&
        isDemoItemShape(value, true) &&
        value.contract_version === 1 &&
        value.dataset_id === "demo-v1" &&
        isNonEmptyTrimmed(value.what_happened) &&
        isNonEmptyTrimmed(value.why_it_matters) &&
        isNonEmptyTrimmed(value.possible_impact) &&
        isStringList(value.facts) &&
        isStringList(value.rule_reasons) &&
        isNonEmptyTrimmed(value.ai_content) &&
        typeof value.ai_confidence_percent === "number" &&
        Number.isInteger(value.ai_confidence_percent) &&
        value.ai_confidence_percent >= 0 &&
        value.ai_confidence_percent <= 100 &&
        isDemoProvenanceV1(value.provenance) &&
        value.publisher === value.provenance.publisher &&
        value.original_url === value.provenance.original_url &&
        value.collected_at === value.provenance.collected_at &&
        (value.provenance.published_at === null ||
            value.published_at === value.provenance.published_at)
    );
}

export function isDemoCatalogV1(value: unknown): value is DemoCatalogV1 {
    return (
        isRecord(value) &&
        value.contract_version === 1 &&
        value.dataset_id === "demo-v1" &&
        Array.isArray(value.items) &&
        value.items.every(isDemoItemV1) &&
        new Set(value.items.map((item) => item.id)).size === value.items.length
    );
}

export function isDemoBootstrapCatalogV1(
    value: unknown,
): value is DemoCatalogV1 {
    return isDemoCatalogV1(value) && value.items.length > 0;
}

export function isDemoPageV1(value: unknown): value is DemoPageV1 {
    return (
        isDemoCatalogV1(value) &&
        isRecord(value) &&
        Object.hasOwn(value, "next_cursor") &&
        (value.next_cursor === null ||
            (typeof value.next_cursor === "string" &&
                value.next_cursor.length <= 1024 &&
                /^v1:[0-9a-f]+:(?:-|[0-9a-f]+):[0-9a-f]+:[0-9a-f]+:[0-9a-f]{16}$/.test(
                    value.next_cursor,
                )))
    );
}

export function isSourceViewV1(value: unknown): value is SourceViewV1 {
    if (
        !isRecord(value) ||
        !hasExactKeys(value, [
            "contract_version",
            "source_id",
            "source_kind",
            "display_url",
            "enabled",
            "revision",
            "created_at",
            "updated_at",
            "last_success_at",
            "freshness",
            "status",
            "retryability",
            "next_allowed_at",
        ])
    )
        return false;
    let displayUrl: URL;
    try {
        displayUrl = new URL(String(value.display_url));
    } catch {
        return false;
    }
    return (
        value.contract_version === 1 &&
        typeof value.source_id === "string" &&
        /^source:[0-9a-f]{24}$/.test(value.source_id) &&
        value.source_kind === "rss_atom" &&
        displayUrl.protocol === "https:" &&
        displayUrl.username === "" &&
        displayUrl.password === "" &&
        displayUrl.search === "" &&
        displayUrl.hash === "" &&
        typeof value.enabled === "boolean" &&
        Number.isSafeInteger(value.revision) &&
        Number(value.revision) >= 1 &&
        typeof value.created_at === "string" &&
        isRfc3339Utc(value.created_at) &&
        typeof value.updated_at === "string" &&
        isRfc3339Utc(value.updated_at) &&
        (value.last_success_at === null ||
            (typeof value.last_success_at === "string" &&
                isRfc3339Utc(value.last_success_at))) &&
        isNullableNonEmptyTrimmed(value.freshness) &&
        ["ready", "error", "retry_wait"].includes(String(value.status)) &&
        ["never", "manual", "automatic", "after"].includes(
            String(value.retryability),
        ) &&
        (value.next_allowed_at === null ||
            (typeof value.next_allowed_at === "string" &&
                isRfc3339Utc(value.next_allowed_at))) &&
        ((value.status === "ready" &&
            value.retryability === "never" &&
            value.next_allowed_at === null) ||
            (value.status === "error" &&
                value.retryability === "never" &&
                value.next_allowed_at === null) ||
            (value.status === "retry_wait" &&
                ["automatic", "after"].includes(String(value.retryability)) &&
                value.next_allowed_at !== null)) &&
        value.created_at <= value.updated_at &&
        (value.last_success_at === null ||
            value.last_success_at <= value.updated_at)
    );
}

export function isSourcePageV1(value: unknown): value is SourcePageV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, ["contract_version", "items", "next_cursor"]) &&
        value.contract_version === 1 &&
        Array.isArray(value.items) &&
        value.items.length <= 100 &&
        value.items.every(isSourceViewV1) &&
        new Set(value.items.map((item) => item.source_id)).size ===
            value.items.length &&
        (value.items.length > 0 || value.next_cursor === null) &&
        (value.next_cursor === null ||
            (typeof value.next_cursor === "string" &&
                /^source-v1:source:[0-9a-f]{24}:[0-9a-f]{16}$/.test(
                    value.next_cursor,
                )))
    );
}

const TASK_STATES: readonly TaskStateV1[] = [
    "queued",
    "running",
    "retry_wait",
    "succeeded",
    "partially_succeeded",
    "failed",
    "cancelled",
];

const ACTIVE_TASK_STATES: readonly TaskStateV1[] = [
    "queued",
    "running",
    "retry_wait",
];

function isSourceId(value: unknown): value is string {
    return typeof value === "string" && /^source:[0-9a-f]{24}$/.test(value);
}

function isTaskId(value: unknown): value is string {
    return typeof value === "string" && /^task:[0-9a-f]{24}$/.test(value);
}

function isTaskState(value: unknown): value is TaskStateV1 {
    return TASK_STATES.includes(value as TaskStateV1);
}

function isOptionalUtc(value: unknown): value is string | null {
    return value === null || (typeof value === "string" && isRfc3339Utc(value));
}

function isOptionalBoundedText(value: unknown): value is string | null {
    return (
        value === null || (isNonEmptyTrimmed(value) && [...value].length <= 512)
    );
}

export function isSyncTargetV1(value: unknown): value is SyncTargetV1 {
    if (!isRecord(value) || typeof value.kind !== "string") return false;
    return value.kind === "all_enabled_rss_atom"
        ? hasExactKeys(value, ["kind"])
        : value.kind === "source_id" &&
              hasExactKeys(value, ["kind", "source_id"]) &&
              isSourceId(value.source_id);
}

export function isStartSyncInputV1(value: unknown): value is StartSyncInputV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "contract_version",
            "target",
            "idempotency_key",
            "foreground_budget_ms",
        ]) &&
        value.contract_version === 1 &&
        isSyncTargetV1(value.target) &&
        isNonEmptyTrimmed(value.idempotency_key) &&
        value.idempotency_key.length <= 128 &&
        /^[\x21-\x7e]+$/.test(value.idempotency_key) &&
        Number.isSafeInteger(value.foreground_budget_ms) &&
        Number(value.foreground_budget_ms) === 30_000
    );
}

export function isTaskRefV1(value: unknown): value is TaskRefV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "contract_version",
            "task_id",
            "state",
            "revision",
        ]) &&
        value.contract_version === 1 &&
        isTaskId(value.task_id) &&
        isTaskState(value.state) &&
        Number.isSafeInteger(value.revision) &&
        Number(value.revision) >= 1
    );
}

export function isSourceSyncStatusV1(
    value: unknown,
): value is SourceSyncStatusV1 {
    if (
        !isRecord(value) ||
        !hasExactKeys(value, [
            "contract_version",
            "source_id",
            "source_revision",
            "state",
            "last_success_at",
            "error_code",
            "next_allowed_at",
            "updated_at",
        ]) ||
        value.contract_version !== 1 ||
        !isSourceId(value.source_id) ||
        !Number.isSafeInteger(value.source_revision) ||
        Number(value.source_revision) < 1 ||
        !isTaskState(value.state) ||
        value.state === "partially_succeeded" ||
        !isOptionalUtc(value.last_success_at) ||
        !isOptionalUtc(value.next_allowed_at) ||
        typeof value.updated_at !== "string" ||
        !isRfc3339Utc(value.updated_at) ||
        !isOptionalBoundedText(value.error_code)
    )
        return false;
    if (
        value.last_success_at !== null &&
        value.last_success_at > value.updated_at
    )
        return false;
    if (value.state === "retry_wait") {
        return value.error_code !== null && value.next_allowed_at !== null;
    }
    if (value.next_allowed_at !== null) return false;
    if (value.state === "failed") return value.error_code !== null;
    if (value.state === "succeeded")
        return value.error_code === null && value.last_success_at !== null;
    if (["queued", "running"].includes(value.state))
        return value.error_code === null;
    return true;
}

export function isTaskSnapshotV1(value: unknown): value is TaskSnapshotV1 {
    if (
        !isRecord(value) ||
        !hasExactKeys(value, [
            "contract_version",
            "task_id",
            "target",
            "state",
            "revision",
            "created_at",
            "started_at",
            "finished_at",
            "updated_at",
            "error_summary",
            "result_ref",
            "sources",
        ]) ||
        value.contract_version !== 1 ||
        !isTaskId(value.task_id) ||
        !isSyncTargetV1(value.target) ||
        !isTaskState(value.state) ||
        !Number.isSafeInteger(value.revision) ||
        Number(value.revision) < 1 ||
        typeof value.created_at !== "string" ||
        !isRfc3339Utc(value.created_at) ||
        !isOptionalUtc(value.started_at) ||
        !isOptionalUtc(value.finished_at) ||
        typeof value.updated_at !== "string" ||
        !isRfc3339Utc(value.updated_at) ||
        !isOptionalBoundedText(value.error_summary) ||
        !(
            value.result_ref === null ||
            (typeof value.result_ref === "string" &&
                /^run:[0-9a-f]{24}$/.test(value.result_ref))
        ) ||
        !Array.isArray(value.sources) ||
        !value.sources.every(isSourceSyncStatusV1) ||
        new Set(value.sources.map((source) => source.source_id)).size !==
            value.sources.length
    )
        return false;
    if (
        value.created_at > value.updated_at ||
        (value.started_at !== null && value.started_at > value.updated_at) ||
        (value.finished_at !== null && value.finished_at > value.updated_at)
    )
        return false;
    if (value.state === "queued") {
        if (
            value.started_at !== null ||
            value.finished_at !== null ||
            value.error_summary !== null
        )
            return false;
    } else if (ACTIVE_TASK_STATES.includes(value.state)) {
        if (value.started_at === null || value.finished_at !== null)
            return false;
        if (value.state === "running" && value.error_summary !== null)
            return false;
    } else if (value.finished_at === null) {
        return false;
    }
    if (
        ["succeeded", "partially_succeeded", "failed"].includes(value.state) &&
        value.started_at === null
    )
        return false;
    if (value.state === "succeeded" && value.error_summary !== null)
        return false;
    if (
        ["partially_succeeded", "failed"].includes(value.state) &&
        value.error_summary === null
    )
        return false;
    const target = value.target;
    if (
        target.kind === "source_id" &&
        value.sources.some((source) => source.source_id !== target.source_id)
    )
        return false;
    if (value.sources.length === 0) return false;
    const sourceStates = value.sources.map((source) => source.state);
    switch (value.state) {
        case "queued":
            return sourceStates.every((state) => state === "queued");
        case "running":
            return (
                sourceStates.some((state) =>
                    ["queued", "running"].includes(state),
                ) &&
                sourceStates.every((state) =>
                    ["queued", "running", "retry_wait"].includes(state),
                )
            );
        case "retry_wait":
            return sourceStates.every((state) => state === "retry_wait");
        case "succeeded":
            return sourceStates.every((state) => state === "succeeded");
        case "partially_succeeded":
            return (
                sourceStates.includes("succeeded") &&
                sourceStates.some((state) =>
                    ["failed", "cancelled", "retry_wait"].includes(state),
                ) &&
                sourceStates.every((state) =>
                    ["succeeded", "failed", "cancelled", "retry_wait"].includes(
                        state,
                    ),
                )
            );
        case "failed":
            return (
                !sourceStates.includes("succeeded") &&
                sourceStates.some((state) =>
                    ["failed", "cancelled"].includes(state),
                ) &&
                sourceStates.every((state) =>
                    ["failed", "cancelled"].includes(state),
                )
            );
        case "cancelled":
            return (
                sourceStates.includes("cancelled") &&
                sourceStates.every((state) =>
                    ["succeeded", "failed", "cancelled"].includes(state),
                )
            );
    }
}

function isSyncResultCountsV1(value: unknown): value is SyncResultCountsV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, ["inserted", "updated", "skipped", "failed"]) &&
        [value.inserted, value.updated, value.skipped, value.failed].every(
            (count) =>
                Number.isSafeInteger(count) &&
                Number(count) >= 0 &&
                Number(count) <= 0xffff_ffff,
        )
    );
}

function isSyncSourceResultV1(value: unknown): value is SyncSourceResultV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "contract_version",
            "source_id",
            "source_revision",
            "source_kind",
            "publisher",
            "status",
            "counts",
            "error_code",
        ]) &&
        value.contract_version === 1 &&
        isSourceId(value.source_id) &&
        Number.isSafeInteger(value.source_revision) &&
        Number(value.source_revision) >= 1 &&
        value.source_kind === "rss_atom" &&
        typeof value.publisher === "string" &&
        value.publisher.length > 0 &&
        value.publisher.length <= 256 &&
        ["retry_wait", "succeeded", "failed", "cancelled"].includes(
            String(value.status),
        ) &&
        isSyncResultCountsV1(value.counts) &&
        isOptionalBoundedText(value.error_code) &&
        (value.status === "succeeded"
            ? Number(value.counts.failed) === 0
                ? value.error_code === null
                : value.error_code !== null
            : value.error_code !== null && Number(value.counts.failed) > 0)
    );
}

function isSyncResultItemV1(value: unknown): value is SyncResultItemV1 {
    if (!isRecord(value)) return false;
    const legacyKeys = [
        "contract_version",
        "result_item_id",
        "sync_run_id",
        "source_id",
        "source_kind",
        "publisher",
        "original_title",
        "published_at",
        "collected_at",
        "original_url",
        "disposition",
    ];
    const currentKeys = [...legacyKeys, "intel_item_id"];
    const isLegacy = hasExactKeys(value, legacyKeys);
    const isCurrent = hasExactKeys(value, currentKeys);
    return (
        (isLegacy || isCurrent) &&
        (isLegacy ||
            value.intel_item_id === null ||
            (typeof value.intel_item_id === "string" &&
                /^intel:[0-9a-f]{64}$/.test(value.intel_item_id))) &&
        value.contract_version === 1 &&
        typeof value.result_item_id === "string" &&
        /^result:[0-9a-f]{24}$/.test(value.result_item_id) &&
        typeof value.sync_run_id === "string" &&
        /^run:[0-9a-f]{24}$/.test(value.sync_run_id) &&
        isSourceId(value.source_id) &&
        value.source_kind === "rss_atom" &&
        typeof value.publisher === "string" &&
        value.publisher.length > 0 &&
        value.publisher.length <= 256 &&
        typeof value.original_title === "string" &&
        value.original_title.trim().length > 0 &&
        isOptionalUtc(value.published_at) &&
        typeof value.collected_at === "string" &&
        isRfc3339Utc(value.collected_at) &&
        typeof value.original_url === "string" &&
        (() => {
            try {
                const url = new URL(value.original_url);
                return ["http:", "https:"].includes(url.protocol);
            } catch {
                return false;
            }
        })() &&
        ["inserted", "updated"].includes(String(value.disposition))
    );
}

interface DecodedResultCursor {
    readonly syncRunId: string;
    readonly sourceId: string;
    readonly collectedAt: string;
    readonly resultItemId: string;
}

function decodeResultCursor(value: string): DecodedResultCursor | null {
    if (!/^cursor:(?:[0-9a-f]{2})+$/.test(value) || value.length > 1031) {
        return null;
    }
    try {
        const hex = value.slice("cursor:".length);
        const bytes = new Uint8Array(hex.length / 2);
        for (let index = 0; index < bytes.length; index += 1) {
            bytes[index] = Number.parseInt(
                hex.slice(index * 2, index * 2 + 2),
                16,
            );
        }
        const decoded = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
        const [syncRunId, sourceId, collectedAt, resultItemId, ...extra] =
            decoded.split("\u001f");
        if (
            extra.length > 0 ||
            !/^run:[0-9a-f]{24}$/.test(syncRunId ?? "") ||
            !isSourceId(sourceId) ||
            !isRfc3339Utc(collectedAt ?? "") ||
            !/^result:[0-9a-f]{24}$/.test(resultItemId ?? "")
        ) {
            return null;
        }
        return { syncRunId, sourceId, collectedAt, resultItemId };
    } catch {
        return null;
    }
}

function addCounts(
    left: SyncResultCountsV1,
    right: SyncResultCountsV1,
): SyncResultCountsV1 | null {
    const sum = {
        inserted: left.inserted + right.inserted,
        updated: left.updated + right.updated,
        skipped: left.skipped + right.skipped,
        failed: left.failed + right.failed,
    };
    return isSyncResultCountsV1(sum) ? sum : null;
}

export function isSyncResultPageV1(value: unknown): value is SyncResultPageV1 {
    if (
        !isRecord(value) ||
        !hasExactKeys(value, [
            "contract_version",
            "summary",
            "items",
            "next_cursor",
        ]) ||
        value.contract_version !== 1 ||
        !isRecord(value.summary) ||
        !hasExactKeys(value.summary, [
            "contract_version",
            "sync_run_id",
            "task_id",
            "outcome",
            "started_at",
            "finished_at",
            "counts",
            "sources",
        ]) ||
        value.summary.contract_version !== 1 ||
        typeof value.summary.sync_run_id !== "string" ||
        !/^run:[0-9a-f]{24}$/.test(value.summary.sync_run_id) ||
        !isTaskId(value.summary.task_id) ||
        ![
            "succeeded_with_results",
            "succeeded_zero_results",
            "partially_succeeded",
            "failed",
        ].includes(String(value.summary.outcome)) ||
        typeof value.summary.started_at !== "string" ||
        !isRfc3339Utc(value.summary.started_at) ||
        typeof value.summary.finished_at !== "string" ||
        !isRfc3339Utc(value.summary.finished_at) ||
        compareRfc3339Utc(value.summary.started_at, value.summary.finished_at) >
            0 ||
        !isSyncResultCountsV1(value.summary.counts) ||
        !Array.isArray(value.summary.sources) ||
        value.summary.sources.length === 0 ||
        !value.summary.sources.every(isSyncSourceResultV1) ||
        !Array.isArray(value.items) ||
        !value.items.every(isSyncResultItemV1) ||
        !(value.next_cursor === null || typeof value.next_cursor === "string")
    )
        return false;
    const summary = value.summary as unknown as SyncResultSummaryV1;
    const items = value.items as readonly SyncResultItemV1[];
    const sources = summary.sources;
    const sourceIds = new Set(sources.map((source) => source.source_id));
    if (sourceIds.size !== sources.length) return false;
    const sourceTotals = sources.reduce<SyncResultCountsV1 | null>(
        (total, source) => (total ? addCounts(total, source.counts) : null),
        { inserted: 0, updated: 0, skipped: 0, failed: 0 },
    );
    if (
        sourceTotals === null ||
        sourceTotals.inserted !== summary.counts.inserted ||
        sourceTotals.updated !== summary.counts.updated ||
        sourceTotals.skipped !== summary.counts.skipped ||
        sourceTotals.failed !== summary.counts.failed
    )
        return false;
    if (
        items.some((item) => item.sync_run_id !== summary.sync_run_id) ||
        new Set(items.map((item) => item.result_item_id)).size !==
            items.length ||
        items.some((item) => {
            const source = sources.find(
                (candidate) => candidate.source_id === item.source_id,
            );
            return (
                source === undefined ||
                source.source_kind !== item.source_kind ||
                source.publisher !== item.publisher
            );
        }) ||
        items.filter((item) => item.disposition === "inserted").length >
            summary.counts.inserted ||
        items.filter((item) => item.disposition === "updated").length >
            summary.counts.updated
    )
        return false;
    if (value.next_cursor !== null) {
        const cursor = decodeResultCursor(value.next_cursor);
        const last = items.at(-1);
        if (
            cursor === null ||
            last === undefined ||
            cursor.syncRunId !== summary.sync_run_id ||
            cursor.sourceId !== last.source_id ||
            cursor.collectedAt !== last.collected_at ||
            cursor.resultItemId !== last.result_item_id
        )
            return false;
    }
    const hasResults = summary.counts.inserted + summary.counts.updated > 0;
    const succeededSources = sources.filter(
        (source) => source.status === "succeeded",
    ).length;
    if (summary.outcome === "succeeded_with_results")
        return (
            hasResults &&
            summary.counts.failed === 0 &&
            succeededSources === sources.length
        );
    if (summary.outcome === "succeeded_zero_results")
        return (
            !hasResults &&
            summary.counts.failed === 0 &&
            items.length === 0 &&
            succeededSources === sources.length
        );
    if (summary.outcome === "partially_succeeded")
        return summary.counts.failed > 0 && succeededSources > 0;
    return (
        !hasResults &&
        summary.counts.failed > 0 &&
        items.length === 0 &&
        succeededSources === 0
    );
}

function isSourceReadinessV1(value: unknown): value is SourceReadinessV1 {
    if (
        !isRecord(value) ||
        !hasExactKeys(value, [
            "contract_version",
            "source_id",
            "source_kind",
            "status",
            "last_success_at",
            "next_allowed_at",
        ]) ||
        value.contract_version !== 1 ||
        value.source_kind !== "rss_atom" ||
        ![
            "not_configured",
            "available",
            "syncing",
            "rate_limited",
            "failed",
            "disabled",
            "retry_wait",
        ].includes(String(value.status)) ||
        !isOptionalUtc(value.last_success_at) ||
        !isOptionalUtc(value.next_allowed_at)
    )
        return false;
    if (value.status === "not_configured") {
        return (
            value.source_id === null &&
            value.last_success_at === null &&
            value.next_allowed_at === null
        );
    }
    if (!isSourceId(value.source_id)) return false;
    if (value.status === "available") {
        return value.last_success_at !== null && value.next_allowed_at === null;
    }
    if (["rate_limited", "retry_wait"].includes(String(value.status))) {
        return value.next_allowed_at !== null;
    }
    if (value.status === "disabled") return true;
    return value.next_allowed_at === null;
}

function isSourceDeliveryReadinessV1(
    value: unknown,
): value is SourceDeliveryReadinessV1 {
    if (
        !isRecord(value) ||
        !hasExactKeys(value, [
            "contract_version",
            "required_source_kinds",
            "status",
            "sources",
        ]) ||
        value.contract_version !== 1 ||
        !Array.isArray(value.required_source_kinds) ||
        value.required_source_kinds.length !== 1 ||
        value.required_source_kinds[0] !== "rss_atom" ||
        !["not_configured", "ready", "syncing", "blocked"].includes(
            String(value.status),
        ) ||
        !Array.isArray(value.sources) ||
        !value.sources.every(isSourceReadinessV1) ||
        new Set(value.sources.map((source) => source.source_id)).size !==
            value.sources.length
    )
        return false;
    if (value.status === "not_configured") {
        return (
            value.sources.length > 0 &&
            value.sources.every((source) =>
                ["not_configured", "disabled"].includes(source.status),
            )
        );
    }
    if (value.sources.length === 0) return false;
    if (value.status === "ready") {
        return value.sources.some((source) => source.status === "available");
    }
    if (value.status === "syncing") {
        return value.sources.some((source) => source.status === "syncing");
    }
    return value.sources.some((source) =>
        ["rate_limited", "retry_wait", "failed"].includes(source.status),
    );
}

export function isSyncHealthSummaryV1(
    value: unknown,
): value is SyncHealthSummaryV1 {
    if (!(
        isRecord(value) &&
        hasExactKeys(value, [
            "contract_version",
            "latest_task",
            "pending_task_count",
            "last_success_at",
            "freshness",
            "source_results",
            "readiness",
        ]) &&
        value.contract_version === 1 &&
        (value.latest_task === null || isTaskSnapshotV1(value.latest_task)) &&
        Number.isSafeInteger(value.pending_task_count) &&
        Number(value.pending_task_count) >= 0 &&
        isOptionalUtc(value.last_success_at) &&
        isNullableNonEmptyTrimmed(value.freshness) &&
        Array.isArray(value.source_results) &&
        value.source_results.every(isSourceSyncStatusV1) &&
        new Set(value.source_results.map((source) => source.source_id)).size ===
            value.source_results.length &&
        isSourceDeliveryReadinessV1(value.readiness)
    ))
        return false;
    const latestTask = value.latest_task;
    const pendingCount = Number(value.pending_task_count);
    if (
        pendingCount === 0 &&
        latestTask !== null &&
        isActiveTaskState(latestTask.state)
    )
        return false;
    if (
        pendingCount > 0 &&
        (latestTask === null || !isActiveTaskState(latestTask.state))
    )
        return false;
    if (latestTask === null) return value.source_results.length === 0;
    const bySource = new Map(
        value.source_results.map(
            (source) => [source.source_id, source] as const,
        ),
    );
    if (
        !latestTask.sources.every((source) => {
            const aggregate = bySource.get(source.source_id);
            return (
                aggregate !== undefined &&
                sameSourceSyncStatus(aggregate, source)
            );
        })
    )
        return false;
    return (
        pendingCount > 0 ||
        value.source_results.length === latestTask.sources.length
    );
}

function isActiveTaskState(state: TaskStateV1) {
    return ["queued", "running", "retry_wait"].includes(state);
}

function sameSourceSyncStatus(
    left: SourceSyncStatusV1,
    right: SourceSyncStatusV1,
) {
    return (
        left.contract_version === right.contract_version &&
        left.source_id === right.source_id &&
        left.source_revision === right.source_revision &&
        left.state === right.state &&
        left.last_success_at === right.last_success_at &&
        left.error_code === right.error_code &&
        left.next_allowed_at === right.next_allowed_at &&
        left.updated_at === right.updated_at
    );
}

function isIntelFeedStreamV1(value: unknown): value is IntelFeedStreamV1 {
    return value === "high_value" || value === "ordinary_candidate";
}

function isCanonicalStringSet(
    value: unknown,
    maximum: number,
    predicate: (entry: string) => boolean,
): value is readonly string[] {
    if (
        !Array.isArray(value) ||
        value.length > maximum ||
        !value.every((entry) => typeof entry === "string" && predicate(entry))
    )
        return false;
    return value.every(
        (entry, index) => index === 0 || value[index - 1] < entry,
    );
}

function isIntelFeedFiltersV1(value: unknown): value is IntelFeedFiltersV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "track_ids",
            "source_ids",
            "time_window",
            "importance",
        ]) &&
        isCanonicalStringSet(value.track_ids, 32, isOpaqueTrackId) &&
        isCanonicalStringSet(value.source_ids, 64, (entry) =>
            /^source:[0-9a-f]{24}$/.test(entry),
        ) &&
        ["all_time", "last_24h", "last_7d", "last_30d"].includes(
            String(value.time_window),
        ) &&
        isCanonicalStringSet(value.importance, 3, (entry) =>
            ["low", "medium", "high"].includes(entry),
        )
    );
}

function isIntelFeedCursor(value: unknown): value is string | null {
    return (
        value === null ||
        (typeof value === "string" &&
            value.length <= 1024 &&
            /^feed-v1:(?:[0-9a-f]{2})+:[0-9a-f]{64}$/.test(value))
    );
}

export function isQueryIntelFeedInputV1(
    value: unknown,
): value is QueryIntelFeedInputV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "contract_version",
            "stream",
            "filters",
            "sort",
            "cursor",
            "limit",
        ]) &&
        value.contract_version === 1 &&
        isIntelFeedStreamV1(value.stream) &&
        isIntelFeedFiltersV1(value.filters) &&
        value.sort === "score_desc" &&
        isIntelFeedCursor(value.cursor) &&
        Number.isSafeInteger(value.limit) &&
        Number(value.limit) >= 1 &&
        Number(value.limit) <= 100
    );
}

function isIntelFeedItemV1(value: unknown): value is IntelFeedItemV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "contract_version",
            "intel_item_id",
            "source_id",
            "source_kind",
            "publisher",
            "title",
            "source_excerpt",
            "excerpt_truncated",
            "published_at",
            "collected_at",
            "importance",
            "score",
            "matched_track_ids",
            "stream_disposition",
            "ai_status",
        ]) &&
        value.contract_version === 1 &&
        typeof value.intel_item_id === "string" &&
        /^intel:[0-9a-f]{64}$/.test(value.intel_item_id) &&
        isSourceId(value.source_id) &&
        value.source_kind === "rss_atom" &&
        typeof value.publisher === "string" &&
        value.publisher.trim().length > 0 &&
        [...value.publisher].length <= 253 &&
        typeof value.title === "string" &&
        value.title.trim().length > 0 &&
        [...value.title].length <= 1024 &&
        (value.source_excerpt === null ||
            (typeof value.source_excerpt === "string" &&
                [...value.source_excerpt].length <= 280)) &&
        typeof value.excerpt_truncated === "boolean" &&
        !(value.source_excerpt === null && value.excerpt_truncated) &&
        isOptionalUtc(value.published_at) &&
        typeof value.collected_at === "string" &&
        isRfc3339Utc(value.collected_at) &&
        ["low", "medium", "high"].includes(String(value.importance)) &&
        Number.isSafeInteger(value.score) &&
        Number(value.score) >= 0 &&
        Number(value.score) <= 100 &&
        isCanonicalStringSet(value.matched_track_ids, 32, isOpaqueTrackId) &&
        isIntelFeedStreamV1(value.stream_disposition) &&
        value.ai_status === "unavailable"
    );
}

export function isIntelFeedPageV1(value: unknown): value is IntelFeedPageV1 {
    if (
        !isRecord(value) ||
        !hasExactKeys(value, [
            "contract_version",
            "stream",
            "filters",
            "sort",
            "rule_version",
            "configuration_revision",
            "configuration_hash",
            "as_of_ms",
            "items",
            "next_cursor",
        ]) ||
        value.contract_version !== 1 ||
        !isIntelFeedStreamV1(value.stream) ||
        !isIntelFeedFiltersV1(value.filters) ||
        value.sort !== "score_desc" ||
        value.rule_version !== "rss-intelligence-value-v1" ||
        !Number.isSafeInteger(value.configuration_revision) ||
        Number(value.configuration_revision) < 1 ||
        typeof value.configuration_hash !== "string" ||
        !/^[0-9a-f]{64}$/.test(value.configuration_hash) ||
        !Number.isSafeInteger(value.as_of_ms) ||
        Number(value.as_of_ms) < 1 ||
        !Array.isArray(value.items) ||
        value.items.length > 100 ||
        !value.items.every(isIntelFeedItemV1) ||
        !isIntelFeedCursor(value.next_cursor)
    )
        return false;
    const items = value.items as readonly IntelFeedItemV1[];
    return (
        new Set(items.map((item) => item.intel_item_id)).size ===
            items.length &&
        items.every((item) => item.stream_disposition === value.stream) &&
        items.every((item, index) => {
            if (index === 0) return true;
            const previous = items[index - 1];
            return (
                previous.score > item.score ||
                (previous.score === item.score &&
                    previous.intel_item_id < item.intel_item_id)
            );
        })
    );
}

function isIntelItemId(value: unknown): value is string {
    return typeof value === "string" && /^intel:[0-9a-f]{64}$/.test(value);
}

function isProvenanceId(value: unknown): value is string {
    return typeof value === "string" && /^[A-Za-z0-9_.:-]{1,128}$/.test(value);
}

export function isQueryIntelEvidenceDetailInputV1(
    value: unknown,
): value is QueryIntelEvidenceDetailInputV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, ["contract_version", "intel_item_id"]) &&
        value.contract_version === 1 &&
        isIntelItemId(value.intel_item_id)
    );
}

export function isOpenIntelOriginalInputV1(
    value: unknown,
): value is OpenIntelOriginalInputV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "contract_version",
            "intel_item_id",
            "provenance_id",
        ]) &&
        value.contract_version === 1 &&
        isIntelItemId(value.intel_item_id) &&
        isProvenanceId(value.provenance_id)
    );
}

function isSourceFactsV1(value: unknown): value is SourceFactsV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "intel_item_id",
            "fact_revision",
            "content_hash",
            "content_state",
            "publisher",
            "title",
            "source_summary",
            "published_at",
            "collected_at",
        ]) &&
        isIntelItemId(value.intel_item_id) &&
        Number.isSafeInteger(value.fact_revision) &&
        Number(value.fact_revision) >= 1 &&
        typeof value.content_hash === "string" &&
        /^[0-9a-f]{64}$/.test(value.content_hash) &&
        value.content_state === "metadata_only" &&
        isBoundedText(value.publisher, 2_048, false) &&
        isBoundedText(value.title, 2_048, false) &&
        (value.source_summary === null ||
            isBoundedText(value.source_summary, 16_384, true)) &&
        isOptionalUtc(value.published_at) &&
        typeof value.collected_at === "string" &&
        isRfc3339Utc(value.collected_at)
    );
}

function isRuleFactorV1(value: unknown): value is RuleFactorV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, ["factor", "points", "reason_codes"]) &&
        [
            "track",
            "source_trust",
            "freshness",
            "technical_impact",
            "user_rule",
        ].includes(String(value.factor)) &&
        isSafePercent(value.points) &&
        isCanonicalStringSet(value.reason_codes, 16, (reason) =>
            /^[A-Za-z0-9_.:-]{1,128}$/.test(reason),
        )
    );
}

function isFilterReasonV1(value: unknown): value is FilterReasonV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, ["code", "actual", "threshold"]) &&
        typeof value.code === "string" &&
        /^[A-Za-z0-9_.:-]{1,128}$/.test(value.code) &&
        (value.actual === null || isSafePercent(value.actual)) &&
        (value.threshold === null || isSafePercent(value.threshold))
    );
}

function isRuleExplanationV1(value: unknown): value is RuleExplanationV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "rule_version",
            "configuration_revision",
            "configuration_hash",
            "evaluated_at_ms",
            "score",
            "importance",
            "disposition",
            "matched_track_ids",
            "factors",
            "filter_reasons",
        ]) &&
        value.rule_version === "rss-intelligence-value-v1" &&
        Number.isSafeInteger(value.configuration_revision) &&
        Number(value.configuration_revision) >= 1 &&
        typeof value.configuration_hash === "string" &&
        /^[0-9a-f]{64}$/.test(value.configuration_hash) &&
        Number.isSafeInteger(value.evaluated_at_ms) &&
        Number(value.evaluated_at_ms) >= 1 &&
        isSafePercent(value.score) &&
        ["low", "medium", "high"].includes(String(value.importance)) &&
        isIntelFeedStreamV1(value.disposition) &&
        isCanonicalStringSet(value.matched_track_ids, 32, isOpaqueTrackId) &&
        Array.isArray(value.factors) &&
        value.factors.length <= 16 &&
        value.factors.every(isRuleFactorV1) &&
        Array.isArray(value.filter_reasons) &&
        value.filter_reasons.length <= 32 &&
        value.filter_reasons.every(isFilterReasonV1)
    );
}

function isIntelProvenanceV1(value: unknown): value is IntelProvenanceV1 {
    if (!(
        isRecord(value) &&
        hasExactKeys(value, [
            "provenance_id",
            "intel_item_id",
            "role",
            "source_id",
            "source_kind",
            "publisher",
            "author",
            "author_availability",
            "original_title",
            "display_url",
            "published_at",
            "collected_at",
            "first_discovered_at",
            "last_updated_at",
            "availability_status",
            "can_open_original",
        ]) &&
        isProvenanceId(value.provenance_id) &&
        isIntelItemId(value.intel_item_id) &&
        (value.role === "primary" || value.role === "associated") &&
        isSourceId(value.source_id) &&
        value.source_kind === "rss_atom" &&
        isBoundedText(value.publisher, 2_048, false) &&
        (value.author === null || isBoundedText(value.author, 2_048, false)) &&
        ["available", "unavailable"].includes(
            String(value.author_availability),
        ) &&
        isBoundedText(value.original_title, 2_048, false) &&
        (isSafeDisplayUrl(value.display_url) ||
            value.display_url === "原文地址不可用") &&
        isOptionalUtc(value.published_at) &&
        typeof value.collected_at === "string" &&
        isRfc3339Utc(value.collected_at) &&
        typeof value.first_discovered_at === "string" &&
        isRfc3339Utc(value.first_discovered_at) &&
        typeof value.last_updated_at === "string" &&
        isRfc3339Utc(value.last_updated_at) &&
        ["available", "unavailable"].includes(
            String(value.availability_status),
        ) &&
        typeof value.can_open_original === "boolean"
    ))
        return false;
    return (
        (!value.can_open_original ||
            (value.availability_status === "available" &&
                isSafeDisplayUrl(value.display_url))) &&
        (value.display_url !== "原文地址不可用" || !value.can_open_original)
    );
}

function isAssociationEvidenceV1(
    value: unknown,
): value is AssociationEvidenceV1 {
    if (!(
        isRecord(value) &&
        hasExactKeys(value, [
            "status",
            "issue_code",
            "relation_type",
            "evidence_basis",
            "basis_version",
        ]) &&
        (value.status === "complete" || value.status === "incomplete") &&
        (value.issue_code === null ||
            isBoundedText(value.issue_code, 128, false))
    ))
        return false;
    const noAssociation =
        value.relation_type === null &&
        value.evidence_basis === null &&
        value.basis_version === null;
    const deterministicAssociation =
        value.relation_type === "same_event" &&
        value.evidence_basis === "normalized_original_url" &&
        value.basis_version === 1;
    return (
        (noAssociation || deterministicAssociation) &&
        (value.status === "incomplete"
            ? value.issue_code !== null && deterministicAssociation
            : value.issue_code === null)
    );
}

export function isIntelEvidenceDetailV1(
    value: unknown,
): value is IntelEvidenceDetailV1 {
    if (!(
        isRecord(value) &&
        hasExactKeys(value, [
            "contract_version",
            "facts",
            "rule_status",
            "rule_issue_code",
            "rule",
            "ai_status",
            "provenance",
            "association",
        ]) &&
        value.contract_version === 1 &&
        isSourceFactsV1(value.facts) &&
        ["current", "unavailable", "stale"].includes(
            String(value.rule_status),
        ) &&
        (value.rule_issue_code === null ||
            isBoundedText(value.rule_issue_code, 128, false)) &&
        (value.rule === null || isRuleExplanationV1(value.rule)) &&
        value.ai_status === "unavailable" &&
        Array.isArray(value.provenance) &&
        value.provenance.length >= 1 &&
        value.provenance.length <= 64 &&
        value.provenance.every(isIntelProvenanceV1) &&
        isAssociationEvidenceV1(value.association)
    ))
        return false;
    const provenance = value.provenance as readonly IntelProvenanceV1[];
    const ruleIsConsistent =
        value.rule_status === "current"
            ? value.rule !== null && value.rule_issue_code === null
            : value.rule === null && value.rule_issue_code !== null;
    return (
        ruleIsConsistent &&
        provenance[0].role === "primary" &&
        provenance[0].intel_item_id === value.facts.intel_item_id &&
        provenance[0].publisher === value.facts.publisher &&
        provenance[0].original_title === value.facts.title &&
        provenance[0].published_at === value.facts.published_at &&
        provenance[0].collected_at === value.facts.collected_at &&
        provenance.slice(1).every((entry) => entry.role === "associated") &&
        new Set(provenance.map((entry) => entry.provenance_id)).size ===
            provenance.length &&
        new Set(provenance.map((entry) => entry.intel_item_id)).size ===
            provenance.length &&
        provenance
            .slice(2)
            .every(
                (entry, index) =>
                    provenance[index + 1].intel_item_id < entry.intel_item_id,
            )
    );
}

export function isOpenOriginalReceiptV1(
    value: unknown,
): value is OpenOriginalReceiptV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "contract_version",
            "intel_item_id",
            "provenance_id",
            "status",
        ]) &&
        value.contract_version === 1 &&
        isIntelItemId(value.intel_item_id) &&
        isProvenanceId(value.provenance_id) &&
        value.status === "requested"
    );
}

function isBoundedText(
    value: unknown,
    maximum: number,
    allowEmpty: boolean,
): value is string {
    return (
        typeof value === "string" &&
        value.trim() === value &&
        (allowEmpty || value.length > 0) &&
        [...value].length <= maximum
    );
}

function isSafeDisplayUrl(value: unknown): value is string {
    if (!isHttpsUrl(value)) return false;
    const parsed = new URL(value);
    return (
        parsed.username === "" &&
        parsed.password === "" &&
        parsed.search === "" &&
        parsed.hash === ""
    );
}

function isOpaqueTrackId(value: string): boolean {
    return /^[A-Za-z0-9_.:-]{1,128}$/.test(value);
}

const SETUP_STATUSES: readonly SetupStepStatusV1[] = [
    "not_started",
    "in_progress",
    "skipped",
    "partially_completed",
    "completed",
];

function hasExactKeys(value: Record<string, unknown>, keys: readonly string[]) {
    const actual = Object.keys(value).sort();
    return (
        actual.length === keys.length &&
        actual.every((key, index) => key === [...keys].sort()[index])
    );
}

function isSafePercent(value: unknown): value is number {
    return (
        Number.isSafeInteger(value) &&
        Number(value) >= 0 &&
        Number(value) <= 100
    );
}

function utf8Length(value: string): number {
    return new TextEncoder().encode(value).length;
}

function isCanonicalTimeOfDay(value: unknown): value is string {
    if (typeof value !== "string" || !/^\d{2}:\d{2}$/.test(value)) return false;
    const [hour, minute] = value.split(":").map(Number);
    return hour <= 23 && minute <= 59;
}

function isHttpSourceIdentifier(value: string): boolean {
    try {
        const parsed = new URL(value);
        return (
            ["http:", "https:"].includes(parsed.protocol) &&
            parsed.hostname.length > 0 &&
            parsed.username.length === 0 &&
            parsed.password.length === 0
        );
    } catch {
        return false;
    }
}

export function isAttentionConfigurationV1(
    value: unknown,
): value is AttentionConfigurationV1 {
    if (
        !isRecord(value) ||
        !hasExactKeys(value, [
            "contract_version",
            "tracks",
            "include_expression",
            "exclude_expression",
            "source_preferences",
            "refresh_enabled",
            "refresh_interval_minutes",
            "minimum_trust",
            "maximum_trust",
            "alert_threshold",
            "quiet_hours",
            "notification_frequency",
            "active_from",
            "active_until",
        ])
    )
        return false;
    return (
        value.contract_version === 1 &&
        Array.isArray(value.tracks) &&
        value.tracks.length >= 1 &&
        value.tracks.length <= 32 &&
        value.tracks.every(
            (track) =>
                isRecord(track) &&
                hasExactKeys(track, ["id", "name", "enabled"]) &&
                isNonEmptyTrimmed(track.id) &&
                utf8Length(track.id) <= 128 &&
                /^[A-Za-z0-9_.:-]+$/.test(track.id) &&
                isNonEmptyTrimmed(track.name) &&
                [...track.name].length <= 64 &&
                typeof track.enabled === "boolean",
        ) &&
        new Set(
            value.tracks.map((track) =>
                String((track as Record<string, unknown>).id),
            ),
        ).size === value.tracks.length &&
        new Set(
            value.tracks.map((track) =>
                String(
                    (track as Record<string, unknown>).name,
                ).toLocaleLowerCase("und"),
            ),
        ).size === value.tracks.length &&
        typeof value.include_expression === "string" &&
        utf8Length(value.include_expression) <= 512 &&
        typeof value.exclude_expression === "string" &&
        utf8Length(value.exclude_expression) <= 512 &&
        Array.isArray(value.source_preferences) &&
        value.source_preferences.length >= 1 &&
        value.source_preferences.length <= 64 &&
        value.source_preferences.every(
            (source) =>
                isRecord(source) &&
                hasExactKeys(source, [
                    "source_kind",
                    "identifier",
                    "enabled",
                    "trust",
                ]) &&
                ["rss", "github", "arxiv"].includes(
                    String(source.source_kind),
                ) &&
                isNonEmptyTrimmed(source.identifier) &&
                utf8Length(source.identifier) <= 2_048 &&
                isHttpSourceIdentifier(source.identifier) &&
                typeof source.enabled === "boolean" &&
                isSafePercent(source.trust),
        ) &&
        new Set(
            value.source_preferences.map((source) => {
                const record = source as Record<string, unknown>;
                return `${String(record.source_kind)}\u0000${String(record.identifier)}`;
            }),
        ).size === value.source_preferences.length &&
        typeof value.refresh_enabled === "boolean" &&
        Number.isSafeInteger(value.refresh_interval_minutes) &&
        Number(value.refresh_interval_minutes) >= 15 &&
        Number(value.refresh_interval_minutes) <= 10_080 &&
        isSafePercent(value.minimum_trust) &&
        isSafePercent(value.maximum_trust) &&
        Number(value.minimum_trust) <= Number(value.maximum_trust) &&
        isSafePercent(value.alert_threshold) &&
        isRecord(value.quiet_hours) &&
        hasExactKeys(value.quiet_hours, ["enabled", "start", "end"]) &&
        typeof value.quiet_hours.enabled === "boolean" &&
        isCanonicalTimeOfDay(value.quiet_hours.start) &&
        isCanonicalTimeOfDay(value.quiet_hours.end) &&
        (!value.quiet_hours.enabled ||
            value.quiet_hours.start !== value.quiet_hours.end) &&
        isRecord(value.notification_frequency) &&
        hasExactKeys(value.notification_frequency, [
            "enabled",
            "max_per_24h",
        ]) &&
        typeof value.notification_frequency.enabled === "boolean" &&
        (value.notification_frequency.enabled
            ? Number.isSafeInteger(value.notification_frequency.max_per_24h) &&
              Number(value.notification_frequency.max_per_24h) >= 1 &&
              Number(value.notification_frequency.max_per_24h) <= 100
            : value.notification_frequency.max_per_24h === null) &&
        (value.active_from === null ||
            (typeof value.active_from === "string" &&
                isRfc3339Utc(value.active_from))) &&
        (value.active_until === null ||
            (typeof value.active_until === "string" &&
                isRfc3339Utc(value.active_until)))
    );
}

function isReceipt(value: unknown): value is ConfigurationValidationReceiptV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "token",
            "normalized_config_hash",
            "validator_version",
        ]) &&
        /^[A-Za-z0-9_-]{43}$/.test(String(value.token)) &&
        /^[0-9a-f]{64}$/.test(String(value.normalized_config_hash)) &&
        value.validator_version === "attention-configuration-v1"
    );
}

export function isConfigurationValidationResultV1(
    value: unknown,
): value is ConfigurationValidationResultV1 {
    if (
        !isRecord(value) ||
        !hasExactKeys(value, [
            "contract_version",
            "blocking_errors",
            "narrowing_risks",
            "validator_version",
            "normalized_config_hash",
            "validation_receipt",
        ])
    )
        return false;
    const blockingCodes = [
        "expression_unparseable",
        "value_out_of_range",
        "lower_bound_above_upper_bound",
        "invalid_source_or_unsupported_protocol",
    ];
    const riskCodes = [
        "all_sources_disabled",
        "all_high_trust_candidates_filtered",
    ];
    const blocking =
        Array.isArray(value.blocking_errors) &&
        value.blocking_errors.every(
            (error) =>
                isRecord(error) &&
                hasExactKeys(error, ["field_path", "code", "message_key"]) &&
                isNonEmptyTrimmed(error.field_path) &&
                blockingCodes.includes(String(error.code)) &&
                error.message_key === `configuration.fix.${String(error.code)}`,
        );
    const risks =
        Array.isArray(value.narrowing_risks) &&
        value.narrowing_risks.every(
            (risk) =>
                isRecord(risk) &&
                hasExactKeys(risk, [
                    "code",
                    "condition_key",
                    "consequence_key",
                ]) &&
                riskCodes.includes(String(risk.code)) &&
                risk.condition_key ===
                    `configuration.risk.${String(risk.code)}.condition` &&
                risk.consequence_key ===
                    `configuration.risk.${String(risk.code)}.consequence`,
        );
    const blockingPaths = Array.isArray(value.blocking_errors)
        ? value.blocking_errors.map((error) =>
              isRecord(error) ? String(error.field_path) : "",
          )
        : [];
    const observedRiskCodes = Array.isArray(value.narrowing_risks)
        ? value.narrowing_risks.map((risk) =>
              isRecord(risk) ? String(risk.code) : "",
          )
        : [];
    if (
        !blocking ||
        !risks ||
        new Set(blockingPaths).size !== blockingPaths.length ||
        blockingPaths.some(
            (path, index) => index > 0 && blockingPaths[index - 1] > path,
        ) ||
        new Set(observedRiskCodes).size !== observedRiskCodes.length ||
        observedRiskCodes.some(
            (code, index) =>
                index > 0 &&
                riskCodes.indexOf(observedRiskCodes[index - 1]) >=
                    riskCodes.indexOf(code),
        ) ||
        value.contract_version !== 1 ||
        value.validator_version !== "attention-configuration-v1" ||
        !/^[0-9a-f]{64}$/.test(String(value.normalized_config_hash))
    )
        return false;
    const receipt = value.validation_receipt;
    return (value.blocking_errors as unknown[]).length > 0
        ? (value.narrowing_risks as unknown[]).length === 0 && receipt === null
        : (value.narrowing_risks as unknown[]).length > 0
          ? isReceipt(receipt) &&
            receipt.normalized_config_hash === value.normalized_config_hash &&
            receipt.validator_version === value.validator_version
          : receipt === null;
}

export function isConfigurationViewV1(
    value: unknown,
): value is ConfigurationViewV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, [
            "contract_version",
            "revision",
            "validator_version",
            "normalized_config_hash",
            "configuration",
            "updated_at_ms",
        ]) &&
        value.contract_version === 1 &&
        Number.isSafeInteger(value.revision) &&
        Number(value.revision) >= 1 &&
        value.validator_version === "attention-configuration-v1" &&
        /^[0-9a-f]{64}$/.test(String(value.normalized_config_hash)) &&
        isAttentionConfigurationV1(value.configuration) &&
        Number.isSafeInteger(value.updated_at_ms) &&
        Number(value.updated_at_ms) >= 1
    );
}

function isSetupOption(value: unknown): value is SetupOptionV1 {
    return (
        isRecord(value) &&
        hasExactKeys(value, ["id", "label", "is_demo"]) &&
        isNonEmptyTrimmed(value.id) &&
        value.id.length <= 64 &&
        /^[A-Za-z0-9_.:-]+$/.test(value.id) &&
        isNonEmptyTrimmed(value.label) &&
        typeof value.is_demo === "boolean"
    );
}

function isUniqueOptions(value: unknown): value is readonly SetupOptionV1[] {
    return (
        Array.isArray(value) &&
        value.length > 0 &&
        value.every(isSetupOption) &&
        new Set(value.map((option) => option.id)).size === value.length
    );
}

export function isSetupProgressV1(value: unknown): value is SetupProgressV1 {
    if (
        !isRecord(value) ||
        !hasExactKeys(value, [
            "contract_version",
            "revision",
            "configuration_revision",
            "overall_status",
            "steps",
            "next_step_id",
            "defaults",
            "saved_config",
        ])
    )
        return false;
    if (
        value.contract_version !== 1 ||
        !Number.isSafeInteger(value.revision) ||
        Number(value.revision) < 0 ||
        !Number.isSafeInteger(value.configuration_revision) ||
        Number(value.configuration_revision) < 1 ||
        !SETUP_STATUSES.includes(value.overall_status as SetupStepStatusV1)
    )
        return false;
    if (
        !Array.isArray(value.steps) ||
        value.steps.length !== SETUP_STEP_IDS.length
    )
        return false;
    const steps = value.steps;
    if (
        !steps.every(
            (step): step is SetupStepProgressV1 =>
                isRecord(step) &&
                hasExactKeys(step, [
                    "contract_version",
                    "step_id",
                    "status",
                    "saved_fields_version",
                ]) &&
                step.contract_version === 1 &&
                SETUP_STEP_IDS.includes(step.step_id as SetupStepIdV1) &&
                SETUP_STATUSES.includes(step.status as SetupStepStatusV1) &&
                (step.saved_fields_version === null ||
                    step.saved_fields_version === 1),
        )
    )
        return false;
    if (
        new Set(steps.map((step) => step.step_id)).size !==
        SETUP_STEP_IDS.length
    )
        return false;
    const defaults = value.defaults;
    if (
        !isRecord(defaults) ||
        !hasExactKeys(defaults, [
            "contract_version",
            "fixture_id",
            "default_track_ids",
            "default_source_example_ids",
            "default_refresh_cadence",
            "tracks",
            "source_examples",
            "refresh_cadences",
        ]) ||
        defaults.contract_version !== 1 ||
        defaults.fixture_id !== "setup-defaults-v1" ||
        !isUniqueOptions(defaults.tracks) ||
        !isUniqueOptions(defaults.source_examples) ||
        !isUniqueOptions(defaults.refresh_cadences) ||
        !defaults.source_examples.every((option) => option.is_demo) ||
        !Array.isArray(defaults.default_track_ids) ||
        !Array.isArray(defaults.default_source_example_ids) ||
        !defaults.default_track_ids.every(isNonEmptyTrimmed) ||
        !defaults.default_source_example_ids.every(isNonEmptyTrimmed) ||
        !isNonEmptyTrimmed(defaults.default_refresh_cadence) ||
        new Set(defaults.default_track_ids).size !==
            defaults.default_track_ids.length ||
        new Set(defaults.default_source_example_ids).size !==
            defaults.default_source_example_ids.length ||
        defaults.default_track_ids.length === 0 ||
        defaults.default_source_example_ids.length === 0 ||
        !defaults.default_track_ids.every((id) =>
            (defaults.tracks as readonly SetupOptionV1[]).some(
                (option) => option.id === id,
            ),
        ) ||
        !defaults.default_source_example_ids.every((id) =>
            (defaults.source_examples as readonly SetupOptionV1[]).some(
                (option) => option.id === id,
            ),
        ) ||
        !defaults.refresh_cadences.some(
            (option) => option.id === defaults.default_refresh_cadence,
        )
    )
        return false;
    if (
        !isRecord(value.saved_config) ||
        !hasExactKeys(value.saved_config, [
            "track_ids",
            "source_example_ids",
            "refresh_cadence",
            "ai_data_disclosure_acknowledged",
        ])
    )
        return false;
    const saved = value.saved_config;
    if (
        !Array.isArray(saved.track_ids) ||
        !saved.track_ids.every(isNonEmptyTrimmed) ||
        new Set(saved.track_ids).size !== saved.track_ids.length ||
        !saved.track_ids.every((id) =>
            (defaults.tracks as readonly SetupOptionV1[]).some(
                (option) => option.id === id,
            ),
        ) ||
        !Array.isArray(saved.source_example_ids) ||
        !saved.source_example_ids.every(isNonEmptyTrimmed) ||
        new Set(saved.source_example_ids).size !==
            saved.source_example_ids.length ||
        !saved.source_example_ids.every((id) =>
            (defaults.source_examples as readonly SetupOptionV1[]).some(
                (option) => option.id === id,
            ),
        ) ||
        !(
            saved.refresh_cadence === null ||
            (isNonEmptyTrimmed(saved.refresh_cadence) &&
                defaults.refresh_cadences.some(
                    (option) => option.id === saved.refresh_cadence,
                ))
        ) ||
        typeof saved.ai_data_disclosure_acknowledged !== "boolean"
    )
        return false;
    if (
        !steps.every((step, index) => step.step_id === SETUP_STEP_IDS[index]) ||
        !steps.every((step) =>
            step.status === "completed"
                ? step.saved_fields_version === 1
                : step.saved_fields_version === null,
        )
    )
        return false;
    const next =
        steps.find((step) => step.status !== "completed")?.step_id ?? null;
    const overall: SetupStepStatusV1 = steps.every(
        (step) => step.status === "completed",
    )
        ? "completed"
        : steps.every((step) => step.status === "not_started")
          ? "not_started"
          : steps.some((step) => step.status === "completed")
            ? "partially_completed"
            : steps.some((step) => step.status === "skipped")
              ? "skipped"
              : "in_progress";
    return value.next_step_id === next && value.overall_status === overall;
}
