import { test as base, expect, type Page } from "@playwright/test";

import {
    createConfigurationValidationResult,
    createConfigurationView,
    createDemoCatalog,
    createDemoItem,
    createDemoPage,
    createHealthStatus,
    createIntelFeedItem,
    createIntelFeedPage,
    createSetupProgress,
    createSourcePage,
    createSourceView,
    createSyncHealthSummary,
    createSyncResultPage,
    createTaskSnapshot,
} from "../factories/demo-dto.factory";
import {
    commandError,
    mergeCommandBehaviors,
    response,
    type ExternalCall,
    type TauriCommand,
    type TauriCommandBehavior,
    type TauriCommandOverrides,
    type TauriInvokeCall,
} from "../helpers/tauri-command-mock";

interface DemoAppFixture {
    invokeCalls(): Promise<TauriInvokeCall[]>;
    externalCalls(): Promise<ExternalCall[]>;
    settledDetailIds(): Promise<string[]>;
    setResponse(command: TauriCommand, value: unknown): Promise<void>;
    setError(command: TauriCommand, error: unknown): Promise<void>;
}

interface DemoAppFixtures {
    demoApp: DemoAppFixture;
    tauriCommandOverrides: TauriCommandOverrides;
}

const defaultCatalog = createDemoCatalog();
const defaultIntelEvidenceDetail = JSON.parse(
    readFileSync(
        resolve(
            process.cwd(),
            "contracts/fixtures/intel-detail/phase1-v1.json",
        ),
        "utf8",
    ),
) as Record<string, unknown>;
const defaultDetailsById = Object.fromEntries(
    defaultCatalog.items.map((item) => [
        item.id,
        createDemoItem({
            ...item,
            provenance: {
                ...createDemoItem().provenance,
                publisher: item.publisher,
                original_title: item.title,
                original_url: item.original_url,
                published_at: item.published_at,
                collected_at: item.collected_at,
                first_discovered_at: item.collected_at,
                last_updated_at: item.collected_at,
            },
        }),
    ]),
);

const defaultBehaviors: Record<TauriCommand, TauriCommandBehavior> = {
    health_v1: response(createHealthStatus()),
    demo_bootstrap_v1: response(defaultCatalog),
    demo_search_v1: response(defaultCatalog),
    demo_list_v1: response(createDemoPage()),
    demo_filter_v1: response(createDemoPage()),
    demo_detail_v1: response({ __detailsById: defaultDetailsById }),
    setup_progress_v1: response(createSetupProgress()),
    save_setup_step_v1: response(createSetupProgress()),
    configuration_v1: response({
        __configurationState: "read",
        initialView: createConfigurationView(),
    }),
    validate_configuration_v1: response({
        __configurationState: "validate",
        initialResult: createConfigurationValidationResult(),
    }),
    save_configuration_v1: response({ __configurationState: "save" }),
    save_source_v1: response({
        __sourceState: "save",
        initialSource: createSourceView(),
    }),
    query_sources_v1: response({
        __sourceState: "query",
        initialPage: createSourcePage({ items: [] }),
    }),
    start_sync_v1: response({
        __syncState: "start",
        initialTask: createTaskSnapshot({
            state: "queued",
            revision: 1,
        }),
    }),
    task_v1: response({
        __syncState: "task",
        initialTask: createTaskSnapshot(),
    }),
    sync_health_v1: response({
        __syncState: "health",
        initialHealth: createSyncHealthSummary({
            latest_task: null,
            pending_task_count: 0,
        }),
    }),
    get_sync_result_v1: response({
        __syncResultState: "read",
        initialPages: [createSyncResultPage()],
    }),
    query_intel_feed_v1: response({
        __intelFeedState: "read",
        initialPage: createIntelFeedPage({
            items: [
                createIntelFeedItem(),
                createIntelFeedItem({
                    intel_item_id: `intel:${"2".repeat(64)}`,
                    title: "Quarterly community note",
                    source_excerpt: null,
                    importance: "medium",
                    score: 50,
                    matched_track_ids: [],
                    stream_disposition: "ordinary_candidate",
                }),
            ],
        }),
    }),
    query_intel_evidence_detail_v1: response({
        __intelDetailState: "read",
        initialDetail: defaultIntelEvidenceDetail,
    }),
    open_intel_original_v1: response({
        __intelDetailState: "open",
        initialDetail: defaultIntelEvidenceDetail,
    }),
};

async function readInvokeCalls(page: Page): Promise<TauriInvokeCall[]> {
    return page.evaluate(() =>
        structuredClone(
            (window as Window & { __TEST_TAURI_CALLS__?: TauriInvokeCall[] })
                .__TEST_TAURI_CALLS__ ?? [],
        ),
    );
}

async function readExternalCalls(page: Page): Promise<ExternalCall[]> {
    return page.evaluate(() =>
        structuredClone(
            (window as Window & { __TEST_EXTERNAL_CALLS__?: ExternalCall[] })
                .__TEST_EXTERNAL_CALLS__ ?? [],
        ),
    );
}

export const test = base.extend<DemoAppFixtures>({
    tauriCommandOverrides: [{}, { option: true }],

    demoApp: [
        async ({ page, tauriCommandOverrides }, use, testInfo) => {
            const browserResourceCalls: ExternalCall[] = [];
            const applicationOrigin = new URL(
                process.env.BASE_URL ?? "http://127.0.0.1:4173",
            ).origin;
            await page.context().route("**/*", async (route) => {
                const request = route.request();
                const target = request.url();
                const protocol = new URL(target).protocol;
                if (
                    new URL(target).origin === applicationOrigin ||
                    protocol === "data:" ||
                    protocol === "blob:"
                ) {
                    await route.continue();
                    return;
                }
                browserResourceCalls.push({
                    kind: "browser_resource",
                    target,
                    method: request.method(),
                });
                await route.abort("blockedbyclient");
            });
            const behaviors = mergeCommandBehaviors(
                defaultBehaviors,
                tauriCommandOverrides,
            );

            await page.addInitScript((initialBehaviors) => {
                type Behavior =
                    | { kind: "response"; value: unknown }
                    | { kind: "error"; error: unknown };
                type InvokeCall = {
                    command: string;
                    args: Record<string, unknown> | null;
                };
                type OutboundCall = {
                    kind:
                        | "fetch"
                        | "xhr"
                        | "websocket"
                        | "sendBeacon"
                        | "notification"
                        | "system_browser";
                    target: string;
                    method: string | null;
                };
                type TestWindow = Window &
                    typeof globalThis & {
                        __TAURI_INTERNALS__?: Record<string, unknown>;
                        __TEST_TAURI_BEHAVIORS__: Record<string, Behavior>;
                        __TEST_TAURI_CALLS__: InvokeCall[];
                        __TEST_EXTERNAL_CALLS__: OutboundCall[];
                        __TEST_SETTLED_DETAIL_IDS__: string[];
                    };

                type Configuration = {
                    contract_version: number;
                    tracks: Array<{
                        id: string;
                        name: string;
                        enabled: boolean;
                    }>;
                    include_expression: string;
                    exclude_expression: string;
                    source_preferences: Array<{
                        source_kind: string;
                        identifier: string;
                        enabled: boolean;
                        trust: number;
                    }>;
                    refresh_enabled: boolean;
                    refresh_interval_minutes: number;
                    minimum_trust: number;
                    maximum_trust: number;
                    alert_threshold: number;
                    quiet_hours: {
                        enabled: boolean;
                        start: string;
                        end: string;
                    };
                    notification_frequency: {
                        enabled: boolean;
                        max_per_24h: number | null;
                    };
                    active_from: string | null;
                    active_until: string | null;
                };

                type ConfigurationView = {
                    contract_version: number;
                    revision: number;
                    validator_version: string;
                    normalized_config_hash: string;
                    configuration: Configuration;
                    updated_at_ms: number;
                };

                const configurationStorageKey =
                    "__TEST_DEVICE_CONFIGURATION_V1__";
                const sourceStorageKey = "__TEST_DEVICE_SOURCES_V1__";
                const syncTaskStorageKey = "__TEST_RSS_SYNC_TASK_V1__";
                const syncResultStorageKey = "__TEST_RSS_SYNC_RESULTS_V1__";
                const externalCallsStorageKey = "__TEST_EXTERNAL_CALLS_V1__";

                const compareUtf8 = (left: string, right: string) => {
                    const encoder = new TextEncoder();
                    const leftBytes = encoder.encode(left);
                    const rightBytes = encoder.encode(right);
                    const length = Math.min(
                        leftBytes.length,
                        rightBytes.length,
                    );
                    for (let index = 0; index < length; index += 1) {
                        if (leftBytes[index] !== rightBytes[index]) {
                            return leftBytes[index] - rightBytes[index];
                        }
                    }
                    return leftBytes.length - rightBytes.length;
                };

                const normalizeConfiguration = (
                    configuration: Configuration,
                ): Configuration => ({
                    contract_version: configuration.contract_version,
                    tracks: configuration.tracks
                        .map((track) => ({
                            id: track.id.trim(),
                            name: track.name.trim(),
                            enabled: track.enabled,
                        }))
                        .sort((left, right) => compareUtf8(left.id, right.id)),
                    include_expression: configuration.include_expression.trim(),
                    exclude_expression: configuration.exclude_expression.trim(),
                    source_preferences: configuration.source_preferences
                        .map((source) => ({
                            source_kind: source.source_kind
                                .trim()
                                .toLowerCase(),
                            identifier: source.identifier.trim(),
                            enabled: source.enabled,
                            trust: source.trust,
                        }))
                        .sort(
                            (left, right) =>
                                compareUtf8(
                                    left.source_kind,
                                    right.source_kind,
                                ) ||
                                compareUtf8(left.identifier, right.identifier),
                        ),
                    refresh_enabled: configuration.refresh_enabled,
                    refresh_interval_minutes:
                        configuration.refresh_interval_minutes,
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
                        max_per_24h:
                            configuration.notification_frequency.max_per_24h,
                    },
                    active_from: configuration.active_from,
                    active_until: configuration.active_until,
                });

                const configurationHash = async (
                    configuration: Configuration,
                ) => {
                    const canonical = JSON.stringify(
                        normalizeConfiguration(configuration),
                    );
                    const digest = await crypto.subtle.digest(
                        "SHA-256",
                        new TextEncoder().encode(canonical),
                    );
                    return [...new Uint8Array(digest)]
                        .map((byte) => byte.toString(16).padStart(2, "0"))
                        .join("");
                };

                const readConfigurationView = (
                    initialView: ConfigurationView,
                ): ConfigurationView => {
                    const stored = localStorage.getItem(
                        configurationStorageKey,
                    );
                    return stored
                        ? (JSON.parse(stored) as ConfigurationView)
                        : structuredClone(initialView);
                };

                const writeConfigurationView = (view: ConfigurationView) => {
                    localStorage.setItem(
                        configurationStorageKey,
                        JSON.stringify(view),
                    );
                };

                const testWindow = window as TestWindow;
                testWindow.__TEST_TAURI_BEHAVIORS__ = initialBehaviors;
                testWindow.__TEST_TAURI_CALLS__ = [];
                testWindow.__TEST_SETTLED_DETAIL_IDS__ = [];
                const persistedExternalCalls = localStorage.getItem(
                    externalCallsStorageKey,
                );
                testWindow.__TEST_EXTERNAL_CALLS__ = persistedExternalCalls
                    ? (JSON.parse(persistedExternalCalls) as OutboundCall[])
                    : [];

                const recordExternal = (
                    kind: OutboundCall["kind"],
                    target: string,
                    method: string | null,
                ) => {
                    testWindow.__TEST_EXTERNAL_CALLS__.push({
                        kind,
                        target,
                        method,
                    });
                    localStorage.setItem(
                        externalCallsStorageKey,
                        JSON.stringify(testWindow.__TEST_EXTERNAL_CALLS__),
                    );
                };

                testWindow.__TAURI_INTERNALS__ = {
                    ...(testWindow.__TAURI_INTERNALS__ ?? {}),
                    invoke: async (
                        command: string,
                        args?: Record<string, unknown>,
                    ) => {
                        testWindow.__TEST_TAURI_CALLS__.push({
                            command,
                            args: args ?? null,
                        });
                        const behavior =
                            testWindow.__TEST_TAURI_BEHAVIORS__[command];
                        if (!behavior) {
                            throw new Error(
                                `Unexpected Tauri command in browser test: ${command}`,
                            );
                        }
                        if (behavior.kind === "error") throw behavior.error;
                        const marker =
                            behavior.value && typeof behavior.value === "object"
                                ? (behavior.value as {
                                      __configurationState?: string;
                                      __sourceState?: string;
                                      __syncState?: string;
                                      __syncResultState?: string;
                                      __intelFeedState?: string;
                                      __intelDetailState?: string;
                                      initialView?: ConfigurationView;
                                      initialResult?: Record<string, unknown>;
                                      initialSource?: Record<string, unknown>;
                                      initialPage?: Record<string, unknown>;
                                      initialTask?: Record<string, unknown>;
                                      initialHealth?: Record<string, unknown>;
                                      initialPages?: Record<string, unknown>[];
                                      initialDetail?: Record<string, unknown>;
                                      detailDelayFramesById?: Record<
                                          string,
                                          number
                                      >;
                                  })
                                : null;
                        if (
                            command === "configuration_v1" &&
                            marker?.__configurationState === "read" &&
                            marker.initialView
                        ) {
                            return structuredClone(
                                readConfigurationView(marker.initialView),
                            );
                        }
                        if (
                            command === "query_sources_v1" &&
                            marker?.__sourceState === "query" &&
                            marker.initialPage
                        ) {
                            const stored =
                                localStorage.getItem(sourceStorageKey);
                            return {
                                ...structuredClone(marker.initialPage),
                                items: stored
                                    ? (JSON.parse(stored) as unknown[])
                                    : (marker.initialPage.items ?? []),
                                next_cursor: null,
                            };
                        }
                        if (
                            command === "save_source_v1" &&
                            marker?.__sourceState === "save" &&
                            marker.initialSource
                        ) {
                            const input = args?.input as
                                { url?: string } | undefined;
                            if (!input?.url) {
                                throw new Error(
                                    "save_source_v1 requires source input",
                                );
                            }
                            const displayUrl = new URL(input.url);
                            displayUrl.search = "";
                            displayUrl.hash = "";
                            const sourceDigest = await crypto.subtle.digest(
                                "SHA-256",
                                new TextEncoder().encode(displayUrl.href),
                            );
                            const sourceIdentity = [
                                ...new Uint8Array(sourceDigest),
                            ]
                                .map((byte) =>
                                    byte.toString(16).padStart(2, "0"),
                                )
                                .join("")
                                .slice(0, 24);
                            const source = {
                                ...structuredClone(marker.initialSource),
                                source_id: `source:${sourceIdentity}`,
                                display_url: displayUrl.href,
                            };
                            const stored =
                                localStorage.getItem(sourceStorageKey);
                            const items = stored
                                ? (JSON.parse(stored) as Record<
                                      string,
                                      unknown
                                  >[])
                                : [];
                            localStorage.setItem(
                                sourceStorageKey,
                                JSON.stringify([
                                    source,
                                    ...items.filter(
                                        (item) =>
                                            item.source_id !== source.source_id,
                                    ),
                                ]),
                            );
                            return structuredClone(source);
                        }
                        if (
                            command === "start_sync_v1" &&
                            marker?.__syncState === "start" &&
                            marker.initialTask
                        ) {
                            const input = args?.input as
                                | { target?: Record<string, unknown> }
                                | undefined;
                            if (!input?.target) {
                                throw new Error(
                                    "start_sync_v1 requires a sync target",
                                );
                            }
                            const targetSourceId =
                                input.target.kind === "source_id" &&
                                typeof input.target.source_id === "string"
                                    ? input.target.source_id
                                    : null;
                            const task = {
                                ...structuredClone(marker.initialTask),
                                target: structuredClone(input.target),
                                sources: marker.initialTask.sources.map(
                                    (source) => ({
                                        ...structuredClone(source),
                                        ...(targetSourceId === null
                                            ? {}
                                            : { source_id: targetSourceId }),
                                    }),
                                ),
                            } as Record<string, unknown>;
                            localStorage.setItem(
                                syncTaskStorageKey,
                                JSON.stringify(task),
                            );
                            return {
                                contract_version: task.contract_version,
                                task_id: task.task_id,
                                state: task.state,
                                revision: task.revision,
                            };
                        }
                        if (
                            command === "task_v1" &&
                            marker?.__syncState === "task" &&
                            marker.initialTask
                        ) {
                            const stored =
                                localStorage.getItem(syncTaskStorageKey);
                            const previous = stored
                                ? (JSON.parse(stored) as Record<
                                      string,
                                      unknown
                                  >)
                                : null;
                            const previousTarget = previous?.target as
                                | { kind?: string; source_id?: string }
                                | undefined;
                            const taskSources =
                                previousTarget?.kind === "source_id" &&
                                typeof previousTarget.source_id === "string" &&
                                Array.isArray(marker.initialTask.sources)
                                    ? marker.initialTask.sources.map(
                                          (source) => ({
                                              ...(source as Record<
                                                  string,
                                                  unknown
                                              >),
                                              source_id:
                                                  previousTarget.source_id,
                                          }),
                                      )
                                    : marker.initialTask.sources;
                            const task = {
                                ...structuredClone(marker.initialTask),
                                sources: structuredClone(taskSources),
                                ...(previous
                                    ? {
                                          task_id: previous.task_id,
                                          target: previous.target,
                                      }
                                    : {}),
                            };
                            localStorage.setItem(
                                syncTaskStorageKey,
                                JSON.stringify(task),
                            );
                            return structuredClone(task);
                        }
                        if (
                            command === "sync_health_v1" &&
                            marker?.__syncState === "health" &&
                            marker.initialHealth
                        ) {
                            const stored =
                                localStorage.getItem(syncTaskStorageKey);
                            if (!stored)
                                return structuredClone(marker.initialHealth);
                            const latestTask = JSON.parse(stored) as Record<
                                string,
                                unknown
                            >;
                            const state = String(latestTask.state ?? "failed");
                            const active = [
                                "queued",
                                "running",
                                "retry_wait",
                            ].includes(state);
                            const blocked = [
                                "partially_succeeded",
                                "failed",
                                "cancelled",
                            ].includes(state);
                            return {
                                ...structuredClone(marker.initialHealth),
                                latest_task: latestTask,
                                pending_task_count: active ? 1 : 0,
                                source_results: Array.isArray(
                                    latestTask.sources,
                                )
                                    ? latestTask.sources
                                    : [],
                                readiness: {
                                    ...(marker.initialHealth
                                        .readiness as Record<string, unknown>),
                                    status: active
                                        ? "syncing"
                                        : blocked
                                          ? "blocked"
                                          : "ready",
                                },
                            };
                        }
                        if (command === "get_sync_result_v1") {
                            const input = args?.input as
                                | {
                                      contract_version?: number;
                                      sync_run_id?: string;
                                      cursor?: string | null;
                                      limit?: number;
                                  }
                                | undefined;
                            if (
                                !input ||
                                Object.keys(input).sort().join("|") !==
                                    "contract_version|cursor|limit|sync_run_id" ||
                                input.contract_version !== 1 ||
                                typeof input.sync_run_id !== "string" ||
                                !Number.isInteger(input.limit) ||
                                Number(input.limit) < 1 ||
                                Number(input.limit) > 100
                            ) {
                                throw new Error(
                                    "get_sync_result_v1 requires exact input",
                                );
                            }
                            const persisted =
                                localStorage.getItem(syncResultStorageKey);
                            const initialPages =
                                marker?.__syncResultState === "read" &&
                                marker.initialPages
                                    ? marker.initialPages
                                    : [
                                          behavior.value as Record<
                                              string,
                                              unknown
                                          >,
                                      ];
                            const pages = persisted
                                ? (JSON.parse(persisted) as Record<
                                      string,
                                      unknown
                                  >[])
                                : structuredClone(initialPages);
                            if (!persisted) {
                                localStorage.setItem(
                                    syncResultStorageKey,
                                    JSON.stringify(pages),
                                );
                            }
                            const pageIndex =
                                input.cursor === null
                                    ? 0
                                    : pages.findIndex(
                                          (_page, index) =>
                                              index > 0 &&
                                              pages[index - 1]?.next_cursor ===
                                                  input.cursor,
                                      );
                            const page = pages[pageIndex];
                            const summary = page?.summary as
                                Record<string, unknown> | undefined;
                            const items = page?.items;
                            if (
                                !page ||
                                summary?.sync_run_id !== input.sync_run_id ||
                                !Array.isArray(items) ||
                                items.length > Number(input.limit)
                            ) {
                                throw new Error(
                                    "get_sync_result_v1 rejected run or cursor identity",
                                );
                            }
                            return structuredClone(page);
                        }
                        if (
                            command === "query_intel_evidence_detail_v1" &&
                            marker?.__intelDetailState === "read" &&
                            marker.initialDetail
                        ) {
                            const input = args?.input as
                                | {
                                      contract_version?: number;
                                      intel_item_id?: string;
                                  }
                                | undefined;
                            if (
                                input?.contract_version !== 1 ||
                                typeof input.intel_item_id !== "string" ||
                                !/^intel:[0-9a-f]{64}$/.test(
                                    input.intel_item_id,
                                ) ||
                                Object.keys(input).sort().join("|") !==
                                    "contract_version|intel_item_id"
                            ) {
                                throw new Error(
                                    "query_intel_evidence_detail_v1 requires exact stable identity",
                                );
                            }
                            const detail = structuredClone(
                                marker.initialDetail,
                            );
                            const knownIds = new Set([
                                `intel:${"1".repeat(64)}`,
                                `intel:${"2".repeat(64)}`,
                            ]);
                            if (!knownIds.has(input.intel_item_id)) {
                                throw {
                                    contract_version: 1,
                                    code: "not_found.intel_detail",
                                    category: "not_found",
                                    message_key: "error.not_found",
                                    retryability: "never",
                                    source_id: null,
                                    task_id: null,
                                    details_allowlisted: "{}",
                                    correlation_id: "test-not-found",
                                };
                            }
                            const delayFrames = Math.max(
                                0,
                                Math.min(
                                    30,
                                    marker.detailDelayFramesById?.[
                                        input.intel_item_id
                                    ] ?? 0,
                                ),
                            );
                            for (
                                let frame = 0;
                                frame < delayFrames;
                                frame += 1
                            ) {
                                await new Promise<void>((resolve) =>
                                    requestAnimationFrame(() => resolve()),
                                );
                            }
                            const facts = detail.facts as Record<
                                string,
                                unknown
                            >;
                            facts.intel_item_id = input.intel_item_id;
                            const provenance = detail.provenance as Record<
                                string,
                                unknown
                            >[];
                            provenance[0].intel_item_id = input.intel_item_id;
                            if (
                                input.intel_item_id ===
                                `intel:${"2".repeat(64)}`
                            ) {
                                facts.title = "Quarterly community note";
                                const rule = detail.rule as Record<
                                    string,
                                    unknown
                                >;
                                rule.score = 50;
                                rule.importance = "medium";
                                rule.disposition = "ordinary_candidate";
                                rule.matched_track_ids = [];
                                rule.filter_reasons = [
                                    {
                                        code: "score_below_threshold",
                                        actual: 50,
                                        threshold: 80,
                                    },
                                ];
                                provenance[0].original_title = facts.title;
                            }
                            testWindow.__TEST_SETTLED_DETAIL_IDS__.push(
                                input.intel_item_id,
                            );
                            return detail;
                        }
                        if (
                            command === "open_intel_original_v1" &&
                            marker?.__intelDetailState === "open" &&
                            marker.initialDetail
                        ) {
                            const input = args?.input as
                                | {
                                      contract_version?: number;
                                      intel_item_id?: string;
                                      provenance_id?: string;
                                  }
                                | undefined;
                            const provenance = marker.initialDetail
                                .provenance as Record<string, unknown>[];
                            const allowed = provenance.some(
                                (source) =>
                                    source.provenance_id ===
                                        input?.provenance_id &&
                                    source.can_open_original === true,
                            );
                            if (
                                input?.contract_version !== 1 ||
                                typeof input.intel_item_id !== "string" ||
                                !/^intel:[0-9a-f]{64}$/.test(
                                    input.intel_item_id,
                                ) ||
                                typeof input.provenance_id !== "string" ||
                                !allowed ||
                                Object.keys(input).sort().join("|") !==
                                    "contract_version|intel_item_id|provenance_id"
                            ) {
                                throw new Error(
                                    "open_intel_original_v1 requires exact stable identities",
                                );
                            }
                            recordExternal(
                                "system_browser",
                                `${input.intel_item_id}|${input.provenance_id}`,
                                null,
                            );
                            return {
                                contract_version: 1,
                                intel_item_id: input.intel_item_id,
                                provenance_id: input.provenance_id,
                                status: "requested",
                            };
                        }
                        if (
                            command === "query_intel_feed_v1" &&
                            marker?.__intelFeedState === "read" &&
                            marker.initialPage
                        ) {
                            const input = args?.input as
                                | {
                                      contract_version?: number;
                                      stream?:
                                          "high_value" | "ordinary_candidate";
                                      filters?: Record<string, unknown>;
                                      sort?: string;
                                      cursor?: string | null;
                                      limit?: number;
                                  }
                                | undefined;
                            if (
                                input?.contract_version !== 1 ||
                                !["high_value", "ordinary_candidate"].includes(
                                    String(input.stream),
                                ) ||
                                input.sort !== "score_desc" ||
                                !input.filters ||
                                !Number.isInteger(input.limit) ||
                                Number(input.limit) < 1 ||
                                Number(input.limit) > 100
                            ) {
                                throw new Error(
                                    "query_intel_feed_v1 requires exact input",
                                );
                            }
                            const template = structuredClone(
                                marker.initialPage,
                            );
                            const filters = input.filters as {
                                track_ids: string[];
                                source_ids: string[];
                                time_window: string;
                                importance: string[];
                            };
                            if (
                                !Array.isArray(filters.track_ids) ||
                                !Array.isArray(filters.source_ids) ||
                                !Array.isArray(filters.importance) ||
                                ![
                                    "all_time",
                                    "last_24h",
                                    "last_7d",
                                    "last_30d",
                                ].includes(filters.time_window) ||
                                !(
                                    input.cursor === null ||
                                    typeof input.cursor === "string"
                                )
                            ) {
                                throw new Error(
                                    "query_intel_feed_v1 requires exact filters and cursor",
                                );
                            }
                            const identity = JSON.stringify({
                                stream: input.stream,
                                filters,
                                sort: input.sort,
                                configuration_revision:
                                    template.configuration_revision,
                                configuration_hash: template.configuration_hash,
                                as_of_ms: template.as_of_ms,
                            });
                            const encodeCursor = (offset: number) => {
                                const bytes = new TextEncoder().encode(
                                    JSON.stringify({ identity, offset }),
                                );
                                const encoded = [...bytes]
                                    .map((byte) =>
                                        byte.toString(16).padStart(2, "0"),
                                    )
                                    .join("");
                                return `feed-v1:${encoded}:${"0".repeat(64)}`;
                            };
                            const decodeCursor = (cursor: string | null) => {
                                if (cursor === null) return 0;
                                const match =
                                    /^feed-v1:([0-9a-f]+):[0-9a-f]{64}$/.exec(
                                        cursor,
                                    );
                                if (!match || match[1].length % 2 !== 0)
                                    throw new Error(
                                        "query_intel_feed_v1 rejected cursor",
                                    );
                                const bytes = new Uint8Array(
                                    match[1]
                                        .match(/.{2}/g)
                                        ?.map((value) => parseInt(value, 16)) ??
                                        [],
                                );
                                const decoded = JSON.parse(
                                    new TextDecoder().decode(bytes),
                                ) as { identity?: string; offset?: number };
                                if (
                                    decoded.identity !== identity ||
                                    !Number.isInteger(decoded.offset) ||
                                    Number(decoded.offset) < 0
                                )
                                    throw new Error(
                                        "query_intel_feed_v1 rejected cursor identity",
                                    );
                                return Number(decoded.offset);
                            };
                            const asOf = new Date(
                                Number(template.as_of_ms),
                            ).getTime();
                            const ages: Record<string, number> = {
                                last_24h: 86_400_000,
                                last_7d: 604_800_000,
                                last_30d: 2_592_000_000,
                            };
                            const items = (
                                template.items as Record<string, unknown>[]
                            )
                                .filter(
                                    (item) =>
                                        item.stream_disposition ===
                                        input.stream,
                                )
                                .filter(
                                    (item) =>
                                        filters.track_ids.length === 0 ||
                                        filters.track_ids.some((track) =>
                                            (
                                                item.matched_track_ids as string[]
                                            ).includes(track),
                                        ),
                                )
                                .filter(
                                    (item) =>
                                        filters.source_ids.length === 0 ||
                                        filters.source_ids.includes(
                                            String(item.source_id),
                                        ),
                                )
                                .filter(
                                    (item) =>
                                        filters.importance.length === 0 ||
                                        filters.importance.includes(
                                            String(item.importance),
                                        ),
                                )
                                .filter((item) => {
                                    if (filters.time_window === "all_time")
                                        return true;
                                    const effective = Date.parse(
                                        String(
                                            item.published_at ??
                                                item.collected_at,
                                        ),
                                    );
                                    const age = ages[filters.time_window];
                                    return (
                                        Number.isFinite(effective) &&
                                        effective <= asOf &&
                                        effective >= asOf - age
                                    );
                                })
                                .sort((left, right) => {
                                    const score =
                                        Number(right.score) -
                                        Number(left.score);
                                    return score !== 0
                                        ? score
                                        : String(
                                              left.intel_item_id,
                                          ).localeCompare(
                                              String(right.intel_item_id),
                                          );
                                });
                            const offset = decodeCursor(input.cursor ?? null);
                            const limit = Number(input.limit);
                            const pageItems = items.slice(
                                offset,
                                offset + limit,
                            );
                            const nextOffset = offset + pageItems.length;
                            return {
                                ...template,
                                stream: input.stream,
                                filters: structuredClone(filters),
                                items: pageItems,
                                next_cursor:
                                    nextOffset < items.length
                                        ? encodeCursor(nextOffset)
                                        : null,
                            };
                        }
                        if (command === "validate_configuration_v1") {
                            const input = args?.input as
                                { configuration?: Configuration } | undefined;
                            if (!input?.configuration) {
                                throw new Error(
                                    "validate_configuration_v1 requires configuration input",
                                );
                            }
                            const hash = await configurationHash(
                                input.configuration,
                            );
                            const base =
                                marker?.__configurationState === "validate"
                                    ? marker.initialResult
                                    : (behavior.value as Record<
                                          string,
                                          unknown
                                      >);
                            if (!base) {
                                throw new Error(
                                    "validate_configuration_v1 requires a response contract",
                                );
                            }
                            const result = structuredClone(base) as Record<
                                string,
                                unknown
                            >;
                            result.normalized_config_hash = hash;
                            if (
                                result.validation_receipt &&
                                typeof result.validation_receipt === "object"
                            ) {
                                result.validation_receipt = {
                                    ...(result.validation_receipt as Record<
                                        string,
                                        unknown
                                    >),
                                    normalized_config_hash: hash,
                                    validator_version:
                                        "attention-configuration-v1",
                                };
                            }
                            return result;
                        }
                        if (
                            command === "save_configuration_v1" &&
                            marker?.__configurationState === "save"
                        ) {
                            const input = args?.input as
                                | {
                                      configuration?: Configuration;
                                      expected_revision?: number;
                                      expected_normalized_config_hash?: string;
                                      validation_receipt?: {
                                          normalized_config_hash?: string;
                                      } | null;
                                  }
                                | undefined;
                            const readBehavior =
                                testWindow.__TEST_TAURI_BEHAVIORS__[
                                    "configuration_v1"
                                ];
                            const readMarker =
                                readBehavior.kind === "response" &&
                                readBehavior.value &&
                                typeof readBehavior.value === "object"
                                    ? (readBehavior.value as {
                                          initialView?: ConfigurationView;
                                      })
                                    : null;
                            if (
                                !input?.configuration ||
                                !readMarker?.initialView
                            ) {
                                throw new Error(
                                    "save_configuration_v1 requires configuration state",
                                );
                            }
                            const currentView = readConfigurationView(
                                readMarker.initialView,
                            );
                            const normalized = normalizeConfiguration(
                                input.configuration,
                            );
                            const hash = await configurationHash(normalized);
                            if (
                                input.expected_revision !==
                                    currentView.revision ||
                                input.expected_normalized_config_hash !==
                                    hash ||
                                (input.validation_receipt !== null &&
                                    input.validation_receipt
                                        ?.normalized_config_hash !== hash)
                            ) {
                                throw new Error(
                                    "save_configuration_v1 rejected stale request identity",
                                );
                            }
                            const nextView: ConfigurationView = {
                                contract_version: 1,
                                revision: currentView.revision + 1,
                                validator_version: "attention-configuration-v1",
                                normalized_config_hash: hash,
                                configuration: normalized,
                                updated_at_ms: currentView.updated_at_ms + 1,
                            };
                            writeConfigurationView(nextView);
                            return structuredClone(nextView);
                        }
                        if (
                            command === "demo_detail_v1" &&
                            behavior.value &&
                            typeof behavior.value === "object" &&
                            "__detailsById" in behavior.value
                        ) {
                            const details = (
                                behavior.value as {
                                    __detailsById: Record<string, unknown>;
                                }
                            ).__detailsById;
                            const id =
                                typeof args?.id === "string" ? args.id : "";
                            if (!(id in details)) {
                                throw new Error(
                                    `Unknown demo detail id: ${id}`,
                                );
                            }
                            return structuredClone(details[id]);
                        }
                        return structuredClone(behavior.value);
                    },
                };

                testWindow.fetch = ((
                    input: RequestInfo | URL,
                    init?: RequestInit,
                ) => {
                    const target =
                        input instanceof Request ? input.url : String(input);
                    const method =
                        init?.method ??
                        (input instanceof Request ? input.method : "GET");
                    recordExternal("fetch", target, method);
                    return Promise.reject(
                        new TypeError(
                            "External network is disabled in demo tests.",
                        ),
                    );
                }) as typeof fetch;

                const xhrMetadata = new WeakMap<
                    XMLHttpRequest,
                    { method: string; target: string }
                >();
                XMLHttpRequest.prototype.open = function (
                    method: string,
                    url: string | URL,
                ) {
                    xhrMetadata.set(this, { method, target: String(url) });
                };
                XMLHttpRequest.prototype.send = function () {
                    const metadata = xhrMetadata.get(this) ?? {
                        method: "GET",
                        target: "unknown",
                    };
                    recordExternal("xhr", metadata.target, metadata.method);
                    throw new TypeError(
                        "External network is disabled in demo tests.",
                    );
                };

                const NativeWebSocket = testWindow.WebSocket;
                const BlockedWebSocket = function (
                    this: WebSocket,
                    url: string | URL,
                ) {
                    recordExternal("websocket", String(url), null);
                    throw new TypeError(
                        "External network is disabled in demo tests.",
                    );
                } as unknown as typeof WebSocket;
                Object.setPrototypeOf(
                    BlockedWebSocket.prototype,
                    NativeWebSocket.prototype,
                );
                testWindow.WebSocket = BlockedWebSocket;

                if (testWindow.navigator.sendBeacon) {
                    Object.defineProperty(testWindow.navigator, "sendBeacon", {
                        configurable: true,
                        value: (url: string | URL): boolean => {
                            recordExternal("sendBeacon", String(url), "POST");
                            return false;
                        },
                    });
                }

                if ("Notification" in testWindow) {
                    const NativeNotification = testWindow.Notification;
                    const BlockedNotification = function (
                        this: Notification,
                        title: string,
                    ) {
                        recordExternal("notification", title, "construct");
                        throw new TypeError(
                            "Notification is disabled in demo tests.",
                        );
                    } as unknown as typeof Notification;
                    Object.setPrototypeOf(
                        BlockedNotification.prototype,
                        NativeNotification.prototype,
                    );
                    Object.defineProperty(BlockedNotification, "permission", {
                        configurable: true,
                        get: () => NativeNotification.permission,
                    });
                    Object.defineProperty(
                        BlockedNotification,
                        "requestPermission",
                        {
                            configurable: true,
                            value: async (): Promise<NotificationPermission> => {
                                recordExternal(
                                    "notification",
                                    "requestPermission",
                                    "requestPermission",
                                );
                                return "denied";
                            },
                        },
                    );
                    testWindow.Notification = BlockedNotification;
                }
            }, behaviors);

            const demoApp: DemoAppFixture = {
                invokeCalls: () => readInvokeCalls(page),
                externalCalls: async () => [
                    ...(await readExternalCalls(page)),
                    ...browserResourceCalls,
                ],
                settledDetailIds: () =>
                    page.evaluate(() =>
                        structuredClone(
                            (
                                window as Window & {
                                    __TEST_SETTLED_DETAIL_IDS__?: string[];
                                }
                            ).__TEST_SETTLED_DETAIL_IDS__ ?? [],
                        ),
                    ),
                setResponse: async (command, value) => {
                    const behavior = { kind: "response" as const, value };
                    await page.addInitScript(
                        ({ command, behavior }) => {
                            const target = window as Window & {
                                __TEST_TAURI_BEHAVIORS__: Record<
                                    string,
                                    TauriCommandBehavior
                                >;
                            };
                            target.__TEST_TAURI_BEHAVIORS__[command] = behavior;
                        },
                        { command, behavior },
                    );
                    await page.evaluate(
                        ({ command, value }) => {
                            const target = window as Window & {
                                __TEST_TAURI_BEHAVIORS__?: Record<
                                    string,
                                    TauriCommandBehavior
                                >;
                            };
                            if (target.__TEST_TAURI_BEHAVIORS__) {
                                target.__TEST_TAURI_BEHAVIORS__[command] = {
                                    kind: "response",
                                    value,
                                };
                                if (command === "get_sync_result_v1") {
                                    localStorage.removeItem(
                                        "__TEST_RSS_SYNC_RESULTS_V1__",
                                    );
                                }
                            }
                        },
                        { command, value },
                    );
                },
                setError: async (command, error) => {
                    const behavior = { kind: "error" as const, error };
                    await page.addInitScript(
                        ({ command, behavior }) => {
                            const target = window as Window & {
                                __TEST_TAURI_BEHAVIORS__: Record<
                                    string,
                                    TauriCommandBehavior
                                >;
                            };
                            target.__TEST_TAURI_BEHAVIORS__[command] = behavior;
                        },
                        { command, behavior },
                    );
                    await page.evaluate(
                        ({ command, error }) => {
                            const target = window as Window & {
                                __TEST_TAURI_BEHAVIORS__?: Record<
                                    string,
                                    TauriCommandBehavior
                                >;
                            };
                            if (target.__TEST_TAURI_BEHAVIORS__) {
                                target.__TEST_TAURI_BEHAVIORS__[command] = {
                                    kind: "error",
                                    error,
                                };
                            }
                        },
                        { command, error },
                    );
                },
            };

            await use(demoApp);

            if (!page.isClosed()) {
                const [invokeCalls, pageExternalCalls] = await Promise.all([
                    readInvokeCalls(page),
                    readExternalCalls(page),
                ]);
                const externalCalls = [
                    ...pageExternalCalls,
                    ...browserResourceCalls,
                ];
                await testInfo.attach("tauri-invoke-calls", {
                    body: JSON.stringify(invokeCalls, null, 2),
                    contentType: "application/json",
                });
                await testInfo.attach("external-calls", {
                    body: JSON.stringify(externalCalls, null, 2),
                    contentType: "application/json",
                });
            }
        },
        { auto: true },
    ],
});

export { commandError, expect, response };
export type {
    DemoAppFixture,
    ExternalCall,
    TauriCommand,
    TauriCommandOverrides,
    TauriInvokeCall,
};
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
