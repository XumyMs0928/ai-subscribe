import { invoke } from "@tauri-apps/api/core";

import {
    DesktopContractError,
    DesktopCommandError,
    DesktopTimeoutError,
    type DesktopApi,
    isDemoCatalogV1,
    isDemoBootstrapCatalogV1,
    isDemoEvidenceDetailV1,
    isDemoPageV1,
    isDesktopApiError,
    isHealthStatusV1,
    isConfigurationValidationResultV1,
    isConfigurationViewV1,
    isSetupProgressV1,
    isSourcePageV1,
    isSourceViewV1,
    isStartSyncInputV1,
    isSyncHealthSummaryV1,
    isSyncResultPageV1,
    isTaskRefV1,
    isTaskSnapshotV1,
    isIntelFeedPageV1,
    isQueryIntelFeedInputV1,
} from "./desktop-api";

type ScheduleDesktopTimeout = (
    onTimeout: () => void,
    timeoutMs: number,
) => () => void;

const scheduleDesktopTimeout: ScheduleDesktopTimeout = (
    onTimeout,
    timeoutMs,
) => {
    const handle = setTimeout(onTimeout, timeoutMs);
    return () => clearTimeout(handle);
};

function canonicalConfigurationJson(
    configuration: import("./desktop-api").AttentionConfigurationV1,
): string {
    return JSON.stringify({
        contract_version: configuration.contract_version,
        tracks: configuration.tracks.map((track) => ({
            id: track.id,
            name: track.name,
            enabled: track.enabled,
        })),
        include_expression: configuration.include_expression,
        exclude_expression: configuration.exclude_expression,
        source_preferences: configuration.source_preferences.map((source) => ({
            source_kind: source.source_kind,
            identifier: source.identifier,
            enabled: source.enabled,
            trust: source.trust,
        })),
        refresh_enabled: configuration.refresh_enabled,
        refresh_interval_minutes: configuration.refresh_interval_minutes,
        minimum_trust: configuration.minimum_trust,
        maximum_trust: configuration.maximum_trust,
        alert_threshold: configuration.alert_threshold,
        quiet_hours: {
            enabled: configuration.quiet_hours.enabled,
            start: configuration.quiet_hours.start,
            end: configuration.quiet_hours.end,
        },
        notification_frequency: {
            enabled: configuration.notification_frequency.enabled,
            max_per_24h: configuration.notification_frequency.max_per_24h,
        },
        active_from: configuration.active_from,
        active_until: configuration.active_until,
    });
}

async function configurationHash(
    configuration: import("./desktop-api").AttentionConfigurationV1,
) {
    if (!globalThis.crypto?.subtle) throw new DesktopContractError();
    const digest = await globalThis.crypto.subtle.digest(
        "SHA-256",
        new TextEncoder().encode(canonicalConfigurationJson(configuration)),
    );
    return [...new Uint8Array(digest)]
        .map((byte) => byte.toString(16).padStart(2, "0"))
        .join("");
}

async function assertConfigurationIdentity(
    view: import("./desktop-api").ConfigurationViewV1,
) {
    if (
        (await configurationHash(view.configuration)) !==
        view.normalized_config_hash
    ) {
        throw new DesktopContractError();
    }
}

export function createTauriDesktopApi(
    scheduleTimeout: ScheduleDesktopTimeout = scheduleDesktopTimeout,
): DesktopApi {
    const timeoutMs = 10_000;
    async function invokeChecked<T>(
        command: string,
        args: Record<string, unknown> | undefined,
        guard: (value: unknown) => value is T,
        commandTimeoutMs = timeoutMs,
    ): Promise<T> {
        try {
            let cancelTimeout: (() => void) | undefined;
            const request =
                args === undefined ? invoke(command) : invoke(command, args);
            const response: unknown = await Promise.race([
                request,
                new Promise<never>((_, reject) => {
                    cancelTimeout = scheduleTimeout(
                        () => reject(new DesktopTimeoutError()),
                        commandTimeoutMs,
                    );
                }),
            ]).finally(() => {
                cancelTimeout?.();
            });
            if (!guard(response)) throw new DesktopContractError();
            return response;
        } catch (error) {
            if (
                error instanceof DesktopContractError ||
                error instanceof DesktopTimeoutError
            )
                throw error;
            if (isDesktopApiError(error)) throw new DesktopCommandError(error);
            throw new DesktopContractError();
        }
    }
    return {
        async health() {
            return invokeChecked("health_v1", undefined, isHealthStatusV1);
        },
        async demoBootstrap() {
            return invokeChecked(
                "demo_bootstrap_v1",
                undefined,
                isDemoBootstrapCatalogV1,
            );
        },
        async demoSearch(query, track) {
            return invokeChecked(
                "demo_search_v1",
                { query, track },
                isDemoCatalogV1,
            );
        },
        async demoList(cursor, limit) {
            return invokeChecked(
                "demo_list_v1",
                { cursor, limit },
                isDemoPageV1,
            );
        },
        async demoFilter(track, cursor, limit) {
            return invokeChecked(
                "demo_filter_v1",
                { track, cursor, limit },
                isDemoPageV1,
            );
        },
        async demoDetail(id) {
            const detail = await invokeChecked(
                "demo_detail_v1",
                { id },
                isDemoEvidenceDetailV1,
            );
            if (detail.id !== id) throw new DesktopContractError();
            return detail;
        },
        async setupProgress() {
            return invokeChecked(
                "setup_progress_v1",
                undefined,
                isSetupProgressV1,
            );
        },
        async saveSetupStep(input) {
            const progress = await invokeChecked(
                "save_setup_step_v1",
                { input },
                isSetupProgressV1,
            );
            const savedStep = progress.steps.find(
                (step) => step.step_id === input.step_id,
            );
            const expectedStatus =
                input.action === "save"
                    ? "completed"
                    : input.action === "skip"
                      ? "skipped"
                      : "in_progress";
            if (
                progress.revision !== input.expected_revision + 1 ||
                savedStep?.status !== expectedStatus
            ) {
                throw new DesktopContractError();
            }
            if (input.action === "save") {
                const returnedValues =
                    input.step_id === "tracks"
                        ? progress.saved_config.track_ids
                        : input.step_id === "source_examples"
                          ? progress.saved_config.source_example_ids
                          : input.step_id === "refresh_cadence"
                            ? progress.saved_config.refresh_cadence === null
                                ? []
                                : [progress.saved_config.refresh_cadence]
                            : progress.saved_config
                                    .ai_data_disclosure_acknowledged
                              ? ["acknowledged"]
                              : [];
                if (
                    returnedValues.length !== input.selected_values.length ||
                    !returnedValues.every(
                        (value, index) =>
                            value === input.selected_values[index],
                    )
                ) {
                    throw new DesktopContractError();
                }
            }
            return progress;
        },
        async configuration() {
            const view = await invokeChecked(
                "configuration_v1",
                undefined,
                isConfigurationViewV1,
            );
            await assertConfigurationIdentity(view);
            return view;
        },
        async validateConfiguration(input) {
            return invokeChecked(
                "validate_configuration_v1",
                { input },
                isConfigurationValidationResultV1,
            );
        },
        async saveConfiguration(input) {
            const view = await invokeChecked(
                "save_configuration_v1",
                { input },
                isConfigurationViewV1,
            );
            if (
                view.revision !== input.expected_revision + 1 ||
                view.normalized_config_hash !==
                    input.expected_normalized_config_hash
            ) {
                throw new DesktopContractError();
            }
            await assertConfigurationIdentity(view);
            return view;
        },
        async saveSource(input) {
            const source = await invokeChecked(
                "save_source_v1",
                { input },
                isSourceViewV1,
                45_000,
            );
            const requested = new URL(input.url);
            const displayed = new URL(source.display_url);
            requested.search = "";
            requested.hash = "";
            if (
                input.contract_version !== 1 ||
                input.source_kind !== "rss_atom" ||
                requested.protocol !== "https:" ||
                displayed.protocol !== "https:" ||
                requested.href !== displayed.href
            ) {
                throw new DesktopContractError();
            }
            return source;
        },
        async querySources(cursor, limit) {
            if (
                !Number.isSafeInteger(limit) ||
                limit < 1 ||
                limit > 100 ||
                (cursor !== null &&
                    !/^source-v1:source:[0-9a-f]{24}:[0-9a-f]{16}$/.test(
                        cursor,
                    ))
            ) {
                throw new DesktopContractError();
            }
            const page = await invokeChecked(
                "query_sources_v1",
                { cursor, limit },
                isSourcePageV1,
            );
            if (page.items.length > limit) {
                throw new DesktopContractError();
            }
            return page;
        },
        async startSync(input) {
            if (!isStartSyncInputV1(input)) throw new DesktopContractError();
            return invokeChecked("start_sync_v1", { input }, isTaskRefV1);
        },
        async task(taskId) {
            if (!/^task:[0-9a-f]{24}$/.test(taskId)) {
                throw new DesktopContractError();
            }
            const snapshot = await invokeChecked(
                "task_v1",
                { taskId },
                isTaskSnapshotV1,
            );
            if (snapshot.task_id !== taskId) {
                throw new DesktopContractError();
            }
            return snapshot;
        },
        async syncHealth() {
            return invokeChecked(
                "sync_health_v1",
                undefined,
                isSyncHealthSummaryV1,
            );
        },
        async getSyncResult(input) {
            if (
                !input ||
                typeof input !== "object" ||
                Object.keys(input).sort().join("|") !==
                    "contract_version|cursor|limit|sync_run_id" ||
                input.contract_version !== 1 ||
                !/^run:[0-9a-f]{24}$/.test(input.sync_run_id) ||
                !Number.isSafeInteger(input.limit) ||
                input.limit < 1 ||
                input.limit > 100 ||
                !(
                    input.cursor === null ||
                    (/^cursor:[0-9a-f]+$/.test(input.cursor) &&
                        input.cursor.length <= 1031)
                )
            ) {
                throw new DesktopContractError();
            }
            const page = await invokeChecked(
                "get_sync_result_v1",
                { input },
                isSyncResultPageV1,
            );
            if (
                page.summary.sync_run_id !== input.sync_run_id ||
                page.items.length > input.limit
            ) {
                throw new DesktopContractError();
            }
            return {
                ...page,
                items: page.items.map((item) => ({
                    ...item,
                    intel_item_id: item.intel_item_id ?? null,
                })),
            };
        },
        async queryIntelFeed(input) {
            if (!isQueryIntelFeedInputV1(input)) {
                throw new DesktopContractError();
            }
            const page = await invokeChecked(
                "query_intel_feed_v1",
                { input },
                isIntelFeedPageV1,
            );
            if (
                page.stream !== input.stream ||
                page.sort !== input.sort ||
                page.items.length > input.limit ||
                !sameIntelFeedFilters(page.filters, input.filters)
            ) {
                throw new DesktopContractError();
            }
            return page;
        },
    };
}

function sameIntelFeedFilters(
    left: import("./desktop-api").IntelFeedFiltersV1,
    right: import("./desktop-api").IntelFeedFiltersV1,
): boolean {
    return (
        left.time_window === right.time_window &&
        sameStrings(left.track_ids, right.track_ids) &&
        sameStrings(left.source_ids, right.source_ids) &&
        sameStrings(left.importance, right.importance)
    );
}

function sameStrings(
    left: readonly string[],
    right: readonly string[],
): boolean {
    return (
        left.length === right.length &&
        left.every((value, index) => value === right[index])
    );
}
